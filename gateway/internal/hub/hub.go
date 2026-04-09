package hub

// Hub owns all active Rooms.
//
// Responsibilities:
//   - Create a Room when the first peer joins a room ID
//   - Return an existing Room for subsequent joiners
//   - Destroy a Room when it becomes empty
//   - Provide a thread-safe lookup: roomID → *Room
//
// The Hub itself has NO knowledge of WebSocket connections or CRDT ops.
// It only manages Room lifetimes.
//
// Concurrency: one mutex guards the rooms map.
// All Room-level concurrency is handled inside Room itself.
//
// Usage (from the WS handler):
//
//	room := hub.GetOrCreate(roomID)
//	room.Join(client)

type Hub struct{}
