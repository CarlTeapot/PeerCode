package protocol

// ── Message envelope ────────────────────────────────────────────────────────
//
//	{
//	  "type":      string,   // one of the MsgType constants below
//	  "room":      string,   // room ID
//	  "client_id": string,   // sender's client ID
//	  "seq":       uint64,   // assigned by server on broadcast; 0 on send
//	  "payload":   string    // base64-encoded opaque bytes (CRDT op or snapshot)
//	}
//
// ── Message types ───────────────────────────────────────────────────────────
//
//	"join"      client → server   first frame after WS upgrade; declares room + client ID + is_host
//	"leave"     server → peers    broadcast when a client disconnects
//	"op"        client → server   a single CRDT operation (insert or delete)
//	            server → peers    same op, now stamped with seq
//	"sync_req"  client → server   new peer asks host for full document state
//	            server → host     gateway forwards the request + requester's client_id
//	"sync_res"  host   → server   host replies with encoded snapshot
//	            server → peer     gateway forwards snapshot to the requesting peer
//	"peer_list" server → client   sent after "join"; lists all currently connected client IDs
//	"ping"      client → server   keepalive
//	"pong"      server → client   keepalive reply

const (
	MsgJoin     = "join"
	MsgLeave    = "leave"
	MsgOp       = "op"
	MsgSyncReq  = "sync_req"
	MsgSyncRes  = "sync_res"
	MsgPeerList = "peer_list"
	MsgPing     = "ping"
	MsgPong     = "pong"
)

type Message struct{}
