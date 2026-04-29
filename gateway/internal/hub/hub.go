package hub

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/coder/websocket"

	"gateway/internal/client"
	"gateway/internal/room"
	"gateway/internal/wire"
)

const stagingOpsBuffer = 64

type Hub struct {
	mu    sync.Mutex
	rooms map[string]*room.Room
}

func New() *Hub {
	return &Hub{rooms: make(map[string]*room.Room)}
}

func (h *Hub) newRoomID() string {
	var b [4]byte
	if _, err := rand.Read(b[:]); err != nil {
		return "00000000"
	}
	return hex.EncodeToString(b[:])
}

func (h *Hub) HandleCreateRoom(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	id := h.newRoomID()
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
	}
	return r
}

func (h *Hub) discardIfStale(id string, r *room.Room) {
	h.mu.Lock()
	defer h.mu.Unlock()
	if h.rooms[id] == r {
		delete(h.rooms, id)
	}
}

func (h *Hub) register(c *client.Client) *room.Room {
	for {
		r := h.getOrCreateRoom(c.RoomID)
		if r.Join(c) {
			return r
		}
		h.discardIfStale(c.RoomID, r)
	}
}

func (h *Hub) unregister(c *client.Client, r *room.Room) {
	r.Leave(c, func() { h.discardIfStale(r.ID, r) })
}

func readWSParams(w http.ResponseWriter, r *http.Request) (roomID, clientID string, ok bool) {
	roomID = r.URL.Query().Get("room")
	if roomID == "" {
		http.Error(w, "missing ?room= parameter", http.StatusBadRequest)
		return "", "", false
	}
	clientID = r.URL.Query().Get("client_id")
	if clientID == "" {
		http.Error(w, "missing ?client_id= parameter", http.StatusBadRequest)
		return "", "", false
	}
	return roomID, clientID, true
}

func dispatchFrame(rm *room.Room, sender *client.Client, raw []byte) {
	if _, err := wire.DecodeOpFrame(raw); err != nil {
		log.Printf("[gateway] drop frame room=%s client=%s: %v", sender.RoomID, sender.ID, err)
		return
	}

	msg := room.BroadcastMsg{Sender: sender, Data: raw}

	select {
	case rm.Ops() <- msg:
		return
	default:
	}

	t := time.NewTimer(100 * time.Millisecond)
	defer t.Stop()
	select {
	case rm.Ops() <- msg:
	case <-t.C:
		log.Printf("[gateway] ops buffer full room=%s client=%s; dropping frame", sender.RoomID, sender.ID)
	}
}

func (h *Hub) HandleWS(w http.ResponseWriter, r *http.Request) {
	roomID, clientID, ok := readWSParams(w, r)
	if !ok {
		return
	}

	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		InsecureSkipVerify: true,
	})
	if err != nil {
		log.Printf("[gateway] upgrade failed room=%s: %v", roomID, err)
		return
	}

	c := client.New(clientID, roomID, conn)
	rm := h.register(c)
	log.Printf("[gateway] join  room=%s client=%s size=%d", roomID, clientID, rm.Size())
	defer func() {
		h.unregister(c, rm)
		log.Printf("[gateway] leave room=%s client=%s", roomID, clientID)
	}()

	ctx := r.Context()
	ops := make(chan []byte, stagingOpsBuffer)
	leave := make(chan *client.Client, 1)

	go c.WritePump(ctx)
	go c.ReadPump(ctx, ops, leave)

	for {
		select {
		case raw := <-ops:
			dispatchFrame(rm, c, raw)
		case <-leave:
			return
		case <-ctx.Done():
			return
		}
	}
}
