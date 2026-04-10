package hub

import "net/http"

// Hub owns all active Rooms.
// TODO: implement room lifecycle (create on first join, destroy when empty)
// and thread-safe roomID → *Room lookup.
type Hub struct{}

func New() *Hub { return &Hub{} }

func (h *Hub) HandleWS(w http.ResponseWriter, r *http.Request) {
	http.Error(w, "WebSocket handler not yet implemented", http.StatusNotImplemented)
}
