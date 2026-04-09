package client

// Client represents a single connected WebSocket peer.
//
// Responsibilities:
//   - Own the websocket.Conn
//   - reads frames from the wire, decodes them
//     into protocol.Message, forwards to the Room's ops channel
//   - drains the send channel, writes frames to the wire
//   - Handle graceful disconnect: close send channel → writePump exits →
//     connection closed → Room is notified via leave channel
//
// A Client never talks to other Clients directly.
// All cross-client communication goes through the Room's fan-out channel.
//
// The send channel is buffered (e.g. 256 messages).
// If it fills up (slow reader), the client is dropped — no backpressure
// allowed to stall the fan-out goroutine.
//
// Fields:
//   ID       string            — unique per connection (uuid or random hex)
//   RoomID   string            — which room this client belongs to
//   IsHost   bool              — true if this client created the room
//   conn     *websocket.Conn
//   send     chan []byte        — outbound frame queue
//   room     *room.Room        — back-reference for leave notification

type Client struct{}
