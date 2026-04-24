package hub

// Rooms returns a snapshot of current room IDs. Test-only: the
// production code path never iterates the hub's registry.
func (h *Hub) Rooms() []string {
	h.mu.Lock()
	defer h.mu.Unlock()
	ids := make([]string, 0, len(h.rooms))
	for id := range h.rooms {
		ids = append(ids, id)
	}
	return ids
}
