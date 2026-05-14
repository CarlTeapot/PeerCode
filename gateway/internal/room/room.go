package room

import (
	"errors"
	"log/slog"
	"sync"

	"gateway/internal/client"
	"gateway/internal/wire"
)

var (
	ErrRoomClosed        = errors.New("room closed")
	ErrDuplicateClientID = errors.New("duplicate client_id")
)

const opsBufferSize = 256

type BroadcastMsg struct {
	Sender *client.Client
	Data   []byte
}

type Room struct {
	ID string

	mu      sync.Mutex
	clients map[*client.Client]struct{}
	closed  bool

	latestSnapshot  []byte
	opsLog          [][]byte
	permissions     map[string]bool
	defaultCanWrite bool

	ops  chan BroadcastMsg
	done chan struct{}
}

func New(id string) *Room {
	return &Room{
		ID:              id,
		clients:         make(map[*client.Client]struct{}),
		permissions:     make(map[string]bool),
		defaultCanWrite: false,
		ops:             make(chan BroadcastMsg, opsBufferSize),
		done:            make(chan struct{}),
	}
}

func (r *Room) Join(c *client.Client) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		slog.Warn("join rejected: room already closed", "room_id", r.ID, "client_id", c.ID)
		return ErrRoomClosed
	}
	for existing := range r.clients {
		if existing.ID == c.ID {
			slog.Warn("join rejected: duplicate client_id", "room_id", r.ID, "client_id", c.ID)
			return ErrDuplicateClientID
		}
	}
	r.clients[c] = struct{}{}
	if c.IsHost {
		r.permissions[c.ID] = true
	} else {
		r.permissions[c.ID] = r.defaultCanWrite
	}
	slog.Info("join accepted", "room_id", r.ID, "client_id", c.ID, "can_write", r.permissions[c.ID], "size", len(r.clients))
	return nil
}

func (r *Room) Leave(c *client.Client, onEmpty func()) {
	r.mu.Lock()
	if _, ok := r.clients[c]; !ok {
		slog.Debug("leave ignored: client not present in room", "room_id", r.ID, "client_id", c.ID)
		r.mu.Unlock()
		return
	}
	delete(r.clients, c)
	delete(r.permissions, c.ID)
	c.CloseSend()
	empty := len(r.clients) == 0
	if empty {
		r.closed = true
		close(r.done)
		slog.Info("room marked closed after last client left", "room_id", r.ID)
	}
	r.mu.Unlock()

	if !empty {
		r.BroadcastRoomState()
	}

	if empty && onEmpty != nil {
		slog.Debug("invoking room onEmpty callback", "room_id", r.ID)
		onEmpty()
	}
}

func (r *Room) Ops() chan<- BroadcastMsg { return r.ops }

func (r *Room) Size() int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return len(r.clients)
}

func (r *Room) StoreSnapshot(data []byte) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.latestSnapshot = make([]byte, len(data))
	copy(r.latestSnapshot, data)
	r.opsLog = nil
	slog.Info("snapshot stored", "room_id", r.ID, "bytes", len(data))
}

func (r *Room) AppendOp(data []byte) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.latestSnapshot == nil {
		return
	}
	cp := make([]byte, len(data))
	copy(cp, data)
	r.opsLog = append(r.opsLog, cp)
}

func (r *Room) ReplayTo(c *client.Client) bool {
	r.mu.Lock()
	snap := r.latestSnapshot
	ops := make([][]byte, len(r.opsLog))
	copy(ops, r.opsLog)
	r.mu.Unlock()

	if snap == nil {
		slog.Debug("replay skipped: no snapshot available", "room_id", r.ID, "client_id", c.ID)
		return false
	}

	if !c.Send(snap) {
		slog.Warn("replay: failed to send snapshot to joiner", "room_id", r.ID, "client_id", c.ID)
		return false
	}
	slog.Info("replay: snapshot sent to joiner", "room_id", r.ID, "client_id", c.ID, "snapshot_bytes", len(snap), "buffered_ops", len(ops))

	for i, op := range ops {
		if !c.Send(op) {
			slog.Warn("replay: failed to send buffered op to joiner", "room_id", r.ID, "client_id", c.ID, "op_index", i)
			break
		}
	}
	return true
}

func (r *Room) Run() {
	slog.Info("room loop started", "room_id", r.ID)
	for {
		select {
		case <-r.done:
			slog.Info("room loop stopping; draining queued ops", "room_id", r.ID)
			r.drain()
			slog.Info("room loop stopped", "room_id", r.ID)
			return
		case msg := <-r.ops:
			if wire.IsControlFrame(msg.Data) {
				r.handleControlFrame(msg)
			} else if wire.IsSnapshotFrame(msg.Data) {
				r.StoreSnapshot(msg.Data)
			} else {
				r.AppendOp(msg.Data)
				r.broadcast(msg)
			}
		}
	}
}

func (r *Room) handleControlFrame(msg BroadcastMsg) {
	subType := wire.ControlSubType(msg.Data)
	switch subType {
	case wire.ControlPermissionChange:
		if msg.Sender == nil || !msg.Sender.IsHost {
			slog.Warn("permission change rejected: sender is not host", "room_id", r.ID)
			return
		}
		var change struct {
			TargetClientID string `json:"target_client_id"`
			CanWrite       bool   `json:"can_write"`
		}
		if err := wire.DecodeControlJSON(msg.Data, &change); err != nil {
			slog.Warn("permission change decode failed", "room_id", r.ID, "error", err)
			return
		}
		r.SetPermission(change.TargetClientID, change.CanWrite)
		r.BroadcastRoomState()
		slog.Info("permission updated", "room_id", r.ID, "target", change.TargetClientID, "can_write", change.CanWrite)
	default:
		slog.Warn("unknown control sub-type in room loop", "room_id", r.ID, "sub_type", subType)
	}
}

func (r *Room) SetPermission(clientID string, canWrite bool) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if _, exists := r.permissions[clientID]; exists {
		r.permissions[clientID] = canWrite
	}
}

type PeerInfo struct {
	ClientID string `json:"client_id"`
	Username string `json:"username"`
	IsHost   bool   `json:"is_host"`
	CanWrite bool   `json:"can_write"`
}

type RoomStatePayload struct {
	Peers []PeerInfo `json:"peers"`
}

func (r *Room) roomStatePayload() RoomStatePayload {
	r.mu.Lock()
	defer r.mu.Unlock()
	peers := make([]PeerInfo, 0, len(r.clients))
	for c := range r.clients {
		peers = append(peers, PeerInfo{
			ClientID: c.ID,
			Username: c.Username,
			IsHost:   c.IsHost,
			CanWrite: r.permissions[c.ID],
		})
	}
	return RoomStatePayload{Peers: peers}
}

func (r *Room) BroadcastRoomState() {
	state := r.roomStatePayload()
	frame, err := wire.EncodeControlJSON(wire.ControlRoomState, state)
	if err != nil {
		slog.Error("failed to encode room state", "room_id", r.ID, "error", err)
		return
	}
	targets := r.getPeers(nil)
	for _, c := range targets {
		if !c.Send(frame) {
			slog.Warn("failed to send room state to client", "room_id", r.ID, "client_id", c.ID)
		}
	}
	slog.Debug("room state broadcast", "room_id", r.ID, "peers", len(state.Peers))
}

func (r *Room) broadcast(msg BroadcastMsg) {
	targets := r.getPeers(msg.Sender)

	slog.Debug(
		"broadcast dispatch prepared",
		"room_id", r.ID,
		"sender_id", msg.Sender.ID,
		"targets", len(targets),
		"bytes", len(msg.Data),
	)
	r.sendToPeers(targets, msg.Data, "disconnecting slow client")
}

func (r *Room) BroadcastAll(data []byte) {
	targets := r.getPeers(nil)
	r.sendToPeers(targets, data, "slow client during BroadcastAll; force-closing")
}

func (r *Room) getPeers(exclude *client.Client) []*client.Client {
	r.mu.Lock()
	defer r.mu.Unlock()
	targets := make([]*client.Client, 0, len(r.clients))
	for c := range r.clients {
		if exclude == nil || c != exclude {
			targets = append(targets, c)
		}
	}
	return targets
}

func (r *Room) sendToPeers(targets []*client.Client, data []byte, slowClientLogMsg string) {
	for _, c := range targets {
		if !c.Send(data) {
			slog.Warn(slowClientLogMsg, "room_id", r.ID, "client_id", c.ID)
			c.ForceClose()
		}
	}
}

func (r *Room) drain() {
	drained := 0
	for {
		select {
		case msg := <-r.ops:
			drained++
			r.broadcast(msg)
		default:
			slog.Debug("room drain finished", "room_id", r.ID, "drained_messages", drained)
			return
		}
	}
}
