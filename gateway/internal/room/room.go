package room

import (
	"sync"

	"gateway/internal/client"
)

const opsBufferSize = 256

type Room struct {
	ID string

	mu      sync.Mutex
	clients map[*client.Client]struct{}
	closed  bool

	ops  chan []byte
	done chan struct{}
}

func New(id string) *Room {
	return &Room{
		ID:      id,
		clients: make(map[*client.Client]struct{}),
		ops:     make(chan []byte, opsBufferSize),
		done:    make(chan struct{}),
	}
}

func (r *Room) Join(c *client.Client) bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		return false
	}
	r.clients[c] = struct{}{}
	return true
}

func (r *Room) Leave(c *client.Client, onEmpty func()) {
	r.mu.Lock()
	if _, ok := r.clients[c]; !ok {
		r.mu.Unlock()
		return
	}
	delete(r.clients, c)
	c.CloseSend()
	empty := len(r.clients) == 0
	if empty {
		r.closed = true
		close(r.done)
	}
	r.mu.Unlock()

	if empty && onEmpty != nil {
		onEmpty()
	}
}

func (r *Room) Ops() chan<- []byte { return r.ops }

func (r *Room) Size() int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return len(r.clients)
}

func (r *Room) Run() {
	for {
		select {
		case <-r.done:
			return
		case payload := <-r.ops:
			// TODO(T04): fan out to every client except sender
			_ = payload
		}
	}
}
