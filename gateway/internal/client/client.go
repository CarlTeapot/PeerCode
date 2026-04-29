package client

import (
	"context"

	"github.com/coder/websocket"
)

const sendBufferSize = 256

type Client struct {
	ID     string
	RoomID string
	conn   *websocket.Conn
	send   chan []byte
}

func New(id, roomID string, conn *websocket.Conn) *Client {
	return &Client{
		ID:     id,
		RoomID: roomID,
		conn:   conn,
		send:   make(chan []byte, sendBufferSize),
	}
}

func (c *Client) CloseSend() {
	close(c.send)
}

// returns false without blocking if the send buffer is full
func (c *Client) Send(data []byte) (ok bool) {
	defer func() {
		if recover() != nil {
			ok = false
		}
	}()
	select {
	case c.send <- data:
		return true
	default:
		return false
	}
}

func (c *Client) ForceClose() {
	if c.conn == nil {
		return
	}
	go c.conn.Close(websocket.StatusPolicyViolation, "slow consumer")
}

// reads frames from the websocket and pushes each payload onto
func (c *Client) ReadPump(ctx context.Context, ops chan<- []byte, leave chan<- *Client) {
	defer func() {
		select {
		case leave <- c:
		case <-ctx.Done():
		}
	}()

	for {
		_, data, err := c.conn.Read(ctx)
		if err != nil {
			return
		}
		select {
		case ops <- data:
		case <-ctx.Done():
			return
		}
	}
}

// drains the send channel to the websocket until it is closed
func (c *Client) WritePump(ctx context.Context) {
	defer func() {
		_ = c.conn.Close(websocket.StatusNormalClosure, "")
	}()

	for {
		select {
		case msg, ok := <-c.send:
			if !ok {
				return
			}
			if err := c.conn.Write(ctx, websocket.MessageBinary, msg); err != nil {
				return
			}
		case <-ctx.Done():
			return
		}
	}
}
