package hub

import (
	"fmt"
	"net/http"

	"github.com/coder/websocket"

	"gateway/internal/wire"
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

	fmt.Printf("[gateway] client connected  room=%s\n", roomID)

	ctx := r.Context()
	defer func() {
		_ = conn.Close(websocket.StatusNormalClosure, "")
		conn.CloseNow()
	}()

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
			payload, err := wire.DecodeOpFrame(msg)
			if err != nil {
				fmt.Printf("[gateway] drop frame room=%s: %v\n", roomID, err)
				continue
			}
			fmt.Printf("[gateway] op    room=%s  payload=%d bytes\n", roomID, len(payload))

			if err := conn.Write(ctx, msgType, msg); err != nil {
				fmt.Printf("[gateway] write error room=%s: %v\n", roomID, err)
				return
			}
		}
	}
}
