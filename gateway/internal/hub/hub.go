package hub

import (
	"crypto/rand"
	"crypto/subtle"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"log/slog"
	"net/http"
	"sync"
	"time"

	"github.com/coder/websocket"

	"gateway/internal/client"
	"gateway/internal/room"
	"gateway/internal/wire"
)

const stagingOpsBuffer = 64
const maxRoomJoinRetries = 3

type Hub struct {
	mu        sync.Mutex
	rooms     map[string]*room.Room
	authToken string
}

func New(authToken string) *Hub {
	return &Hub{rooms: make(map[string]*room.Room), authToken: authToken}
}

func newRoomID() (string, error) {
	var b [4]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "", err
	}
	return hex.EncodeToString(b[:]), nil
}

func (h *Hub) HandleCreateRoom(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		slog.Warn("create room rejected: method not allowed", "method", r.Method)
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	id, err := newRoomID()
	if err != nil {
		slog.Error("create room failed: could not read random bytes", "error", err)
		http.Error(w, "internal server error", http.StatusInternalServerError)
		return
	}
	slog.Info("room id generated", "room_id", id)
	w.Header().Set("Content-Type", "application/json")
	_ = json.NewEncoder(w).Encode(map[string]string{"room_id": id})
}

func (h *Hub) getOrCreateRoom(id string) *room.Room {
	h.mu.Lock()
	defer h.mu.Unlock()
	r, ok := h.rooms[id]
	if !ok {
		r = room.New(id)
		h.rooms[id] = r
		go r.Run()
		slog.Info("room created", "room_id", id)
	} else {
		slog.Debug("room reused", "room_id", id, "size", r.Size())
	}
	return r
}

func (h *Hub) discardIfStale(id string, r *room.Room) {
	h.mu.Lock()
	defer h.mu.Unlock()
	if h.rooms[id] == r {
		delete(h.rooms, id)
		slog.Info("room discarded from hub", "room_id", id)
	} else {
		slog.Debug("discard skipped: room reference is stale", "room_id", id)
	}
}

func (h *Hub) register(c *client.Client) (*room.Room, error) {
	for range maxRoomJoinRetries + 1 {
		r := h.getOrCreateRoom(c.RoomID)
		err := r.Join(c)
		if err == nil {
			slog.Info(
				"client registered to room",
				"room_id", c.RoomID,
				"client_id", c.ID,
				"size", r.Size(),
			)
			return r, nil
		}
		if errors.Is(err, room.ErrDuplicateClientID) {
			slog.Warn(
				"duplicate client_id; rejecting websocket",
				"room_id", c.RoomID,
				"client_id", c.ID,
			)
			return nil, err
		}
		if errors.Is(err, room.ErrRoomClosed) {
			slog.Warn(
				"room join failed; retrying after stale room cleanup",
				"room_id", c.RoomID,
				"client_id", c.ID,
			)
			h.discardIfStale(c.RoomID, r)
			continue
		}
		return nil, err
	}
	slog.Error("register failed: exceeded max retries", "room_id", c.RoomID, "client_id", c.ID)
	return nil, fmt.Errorf("room %s unstable after %d attempts", c.RoomID, maxRoomJoinRetries)
}

func (h *Hub) unregister(c *client.Client, r *room.Room) {
	slog.Debug("unregistering client from room", "room_id", r.ID, "client_id", c.ID)
	r.Leave(c, func() { h.discardIfStale(r.ID, r) })
}

type wsParams struct {
	roomID    string
	clientID  string
	username  string
	hostToken string
}

func readWSParams(w http.ResponseWriter, r *http.Request) (wsParams, bool) {
	roomID := r.URL.Query().Get("room")
	if roomID == "" {
		slog.Warn("websocket rejected: missing room parameter")
		http.Error(w, "missing ?room= parameter", http.StatusBadRequest)
		return wsParams{}, false
	}
	clientID := r.URL.Query().Get("client_id")
	if clientID == "" {
		slog.Warn("websocket rejected: missing client_id parameter", "room_id", roomID)
		http.Error(w, "missing ?client_id= parameter", http.StatusBadRequest)
		return wsParams{}, false
	}
	username := r.URL.Query().Get("username")
	hostToken := r.URL.Query().Get("host_token")
	return wsParams{roomID: roomID, clientID: clientID, username: username, hostToken: hostToken}, true
}

func dispatchFrame(rm *room.Room, sender *client.Client, raw []byte) {
	if err := wire.ValidateFrame(raw); err != nil {
		slog.Warn(
			"dropping invalid frame",
			"room_id", rm.ID,
			"client_id", sender.ID,
			"error", err,
			"bytes", len(raw),
		)
		return
	}

	msg := room.BroadcastMsg{Sender: sender, Data: raw}

	select {
	case rm.Ops() <- msg:
		slog.Debug("frame dispatched to room ops queue", "room_id", rm.ID, "client_id", sender.ID, "bytes", len(raw))
		return
	default:
	}

	t := time.NewTimer(100 * time.Millisecond)
	defer t.Stop()
	select {
	case rm.Ops() <- msg:
		slog.Debug(
			"frame dispatched to room ops queue after short wait",
			"room_id", rm.ID,
			"client_id", sender.ID,
			"bytes", len(raw),
		)
	case <-t.C:
		slog.Warn(
			"room ops buffer full; dropping frame after timeout",
			"room_id", rm.ID,
			"client_id", sender.ID,
			"bytes", len(raw),
		)
	}
}

func (h *Hub) HandleEndSession(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	roomID := r.URL.Query().Get("room")
	if roomID == "" {
		http.Error(w, "missing ?room= parameter", http.StatusBadRequest)
		return
	}
	h.mu.Lock()
	rm, ok := h.rooms[roomID]
	h.mu.Unlock()
	if !ok {
		slog.Warn("end-session: room not found", "room_id", roomID)
		http.Error(w, "room not found", http.StatusNotFound)
		return
	}
	frame := wire.EncodeControlFrame(wire.ControlSessionEnded)
	rm.BroadcastAll(frame)
	slog.Info("end-session: session-ended broadcast sent", "room_id", roomID)
	w.WriteHeader(http.StatusOK)
}

func (h *Hub) HandleWS(w http.ResponseWriter, r *http.Request) {
	params, ok := readWSParams(w, r)
	if !ok {
		return
	}

	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		InsecureSkipVerify: true,
	})
	if err != nil {
		slog.Warn("websocket upgrade failed", "room_id", params.roomID, "error", err)
		return
	}
	slog.Info("websocket upgraded", "room_id", params.roomID, "client_id", params.clientID)

	isHost := params.hostToken != "" &&
		subtle.ConstantTimeCompare([]byte(params.hostToken), []byte(h.authToken)) == 1

	c := client.New(params.clientID, params.roomID, params.username, isHost, conn)
	rm, err := h.register(c)
	if err != nil {
		if errors.Is(err, room.ErrDuplicateClientID) {
			_ = conn.Close(websocket.StatusPolicyViolation, "duplicate client_id")
		} else {
			_ = conn.Close(websocket.StatusInternalError, "join failed")
		}
		return
	}
	slog.Info("client joined room", "room_id", params.roomID, "client_id", params.clientID, "is_host", isHost, "size", rm.Size())
	defer func() {
		h.unregister(c, rm)
		slog.Info("client left room", "room_id", params.roomID, "client_id", params.clientID)
	}()

	if rm.ReplayTo(c) {
		slog.Info("snapshot replayed to joiner", "room_id", params.roomID, "client_id", params.clientID)
	}
	rm.BroadcastRoomState()

	ctx := r.Context()
	ops := make(chan []byte, stagingOpsBuffer)
	leave := make(chan *client.Client, 1)

	go c.WritePump(ctx)
	go c.ReadPump(ctx, ops, leave)

	for {
		select {
		case raw := <-ops:
			slog.Debug("received websocket frame from client", "room_id", params.roomID, "client_id", params.clientID, "bytes", len(raw))
			dispatchFrame(rm, c, raw)
		case <-leave:
			slog.Info("client signaled leave from read pump", "room_id", params.roomID, "client_id", params.clientID)
			return
		case <-ctx.Done():
			slog.Info("websocket handler context cancelled", "room_id", params.roomID, "client_id", params.clientID, "error", ctx.Err())
			return
		}
	}
}
