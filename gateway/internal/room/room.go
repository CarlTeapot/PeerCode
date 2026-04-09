package room

// Room represents one collaborative editing session.
// Every unique room ID gets exactly one Room instance.
//
// Responsibilities:
//   - Track which Client is the host (first joiner, or explicit claim)
//   - Track all connected peer Clients
//   - Run a single fan-out goroutine: reads from ops channel, writes to all peers
//   - Handle join / leave events
//   - Assign a monotonically increasing sequence number to every forwarded op
//     so late-joining peers can detect gaps and request replay
//   - Forward a "sync request" from a new peer directly to the host,
//     so the host can respond with a full document snapshot
//
// The Room never deserializes op payloads. It treats them as opaque []byte.
// Op ordering and conflict resolution are purely the CRDT library's job.
//
// Concurrency model:
//   - One goroutine per Room (the fan-out loop), started in Run()
//   - Join/Leave send signals over channels into that goroutine
//   - No locks needed: all state mutation happens inside the fan-out goroutine
//
// Lifecycle:
//   hub.GetOrCreate → room.Run() in a goroutine → room empties → signals hub to delete

type Room struct{}
