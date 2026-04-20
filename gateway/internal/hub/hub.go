package hub

import (
	"context"
	"fmt"
	"net/http"

	"github.com/coder/websocket"
)

type Hub struct{}

func New() *Hub { return &Hub{} }

func (h *Hub) HandleWS(w http.ResponseWriter, r *http.Request) {
	roomID := r.URL.Query().Get("room")
	if roomID == "" {
		http.Error(w, "missing ?room= parameter", http.StatusBadRequest)
		return
	}

	conn, err := websocket.Accept(w, r, &websocket.AcceptOptions{
		InsecureSkipVerify: true,
	})
	if err != nil {
		fmt.Printf("[gateway] upgrade failed for room %q: %v\n", roomID, err)
		return
	}
	defer func() {
		gracefulClose(ctx, conn, "")
		conn.CloseNow()
	}()

	fmt.Printf("[gateway] client connected  room=%s\n", roomID)

	ctx := r.Context()
	for {
		msgType, msg, err := conn.Read(ctx)
		if err != nil {
			fmt.Printf("[gateway] client disconnected room=%s: %v\n", roomID, err)
			return
		}

		switch msgType {
		case websocket.MessageText:
			fmt.Printf("[gateway] text  room=%s  %s\n", roomID, msg)
		case websocket.MessageBinary:
			fmt.Printf("[gateway] binary room=%s  %d bytes\n", roomID, len(msg))
		}

		if err := conn.Write(ctx, msgType, msg); err != nil {
			fmt.Printf("[gateway] write error room=%s: %v\n", roomID, err)
			return
		}
	}
}
func gracefulClose(ctx context.Context, conn *websocket.Conn, reason string) {
	_ = conn.Close(websocket.StatusNormalClosure, reason)
}
