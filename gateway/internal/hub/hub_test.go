package hub

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"regexp"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/coder/websocket"

	"gateway/internal/wire"
)

func newTestServer(t *testing.T) (*httptest.Server, *Hub) {
	t.Helper()
	h := New()
	mux := http.NewServeMux()
	mux.HandleFunc("/ws", h.HandleWS)
	mux.HandleFunc("/rooms", h.HandleCreateRoom)
	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	return srv, h
}

func wsURL(base, room, clientID string) string {
	return strings.Replace(base, "http://", "ws://", 1) +
		"/ws?room=" + room + "&client_id=" + clientID
}

func dial(t *testing.T, url string) *websocket.Conn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	c, _, err := websocket.Dial(ctx, url, nil)
	if err != nil {
		t.Fatalf("dial %s: %v", url, err)
	}
	return c
}

func TestHub_PostRoomsReturnsFreshID(t *testing.T) {
	srv, _ := newTestServer(t)

	var ids []string
	for i := 0; i < 2; i++ {
		resp, err := http.Post(srv.URL+"/rooms", "application/json", nil)
		if err != nil {
			t.Fatalf("POST /rooms: %v", err)
		}
		var body map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Fatalf("decode: %v", err)
		}
		resp.Body.Close()
		id := body["room_id"]
		if !regexp.MustCompile(`^[0-9a-f]{8}$`).MatchString(id) {
			t.Fatalf("room_id=%q, want 8 hex chars", id)
		}
		ids = append(ids, id)
	}
	if ids[0] == ids[1] {
		t.Fatalf("IDs collided: %s", ids[0])
	}
}

func TestHub_PostRoomsRejectsGet(t *testing.T) {
	srv, _ := newTestServer(t)
	resp, err := http.Get(srv.URL + "/rooms")
	if err != nil {
		t.Fatalf("GET /rooms: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusMethodNotAllowed {
		t.Fatalf("status=%d, want 405", resp.StatusCode)
	}
}

func TestHub_RejectsMissingQuery(t *testing.T) {
	srv, _ := newTestServer(t)
	for _, path := range []string{"/ws", "/ws?room=r", "/ws?client_id=c"} {
		resp, err := http.Get(srv.URL + path)
		if err != nil {
			t.Fatalf("GET %s: %v", path, err)
		}
		resp.Body.Close()
		if resp.StatusCode != http.StatusBadRequest {
			t.Fatalf("%s: status=%d, want 400", path, resp.StatusCode)
		}
	}
}

func TestHub_TwoClientsSameRoomShareSet(t *testing.T) {
	srv, h := newTestServer(t)
	a := dial(t, wsURL(srv.URL, "shared", "alice"))
	defer a.Close(websocket.StatusNormalClosure, "")
	b := dial(t, wsURL(srv.URL, "shared", "bob"))
	defer b.Close(websocket.StatusNormalClosure, "")

	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		if len(h.Rooms()) == 1 {
			break
		}
		time.Sleep(5 * time.Millisecond)
	}
	if rooms := h.Rooms(); len(rooms) != 1 || rooms[0] != "shared" {
		t.Fatalf("Rooms=%v, want [shared]", rooms)
	}
}

func TestHub_DisconnectRemovesClient(t *testing.T) {
	srv, h := newTestServer(t)
	a := dial(t, wsURL(srv.URL, "ephemeral", "alice"))

	registerDeadline := time.Now().Add(time.Second)
	for time.Now().Before(registerDeadline) && len(h.Rooms()) == 0 {
		time.Sleep(5 * time.Millisecond)
	}
	if len(h.Rooms()) != 1 {
		t.Fatalf("room not registered")
	}

	_ = a.Close(websocket.StatusNormalClosure, "bye")

	disconnectDeadline := time.Now().Add(time.Second)
	for time.Now().Before(disconnectDeadline) && len(h.Rooms()) != 0 {
		time.Sleep(5 * time.Millisecond)
	}
	if got := h.Rooms(); len(got) != 0 {
		t.Fatalf("Rooms=%v, want empty after disconnect", got)
	}
}

func TestHub_RoomsIsolated(t *testing.T) {
	srv, _ := newTestServer(t)
	a := dial(t, wsURL(srv.URL, "x", "a"))
	defer a.Close(websocket.StatusNormalClosure, "")
	c := dial(t, wsURL(srv.URL, "y", "c"))
	defer c.Close(websocket.StatusNormalClosure, "")

	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()
	if err := a.Write(ctx, websocket.MessageBinary, wire.EncodeOpFrame([]byte("hello"))); err != nil {
		t.Fatalf("write: %v", err)
	}

	readCtx, readCancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer readCancel()
	_, _, err := c.Read(readCtx)
	if err == nil {
		t.Fatal("client in room y received a frame from room x — rooms are not isolated")
	}
}

func TestHub_ConcurrentJoinsSameRoom(t *testing.T) {
	srv, h := newTestServer(t)
	const n = 10
	var wg sync.WaitGroup
	wg.Add(n)
	conns := make(chan *websocket.Conn, n)
	for i := 0; i < n; i++ {
		go func(i int) {
			defer wg.Done()
			cid := fmt.Sprintf("c%d", i)
			conns <- dial(t, wsURL(srv.URL, "race", cid))
		}(i)
	}
	wg.Wait()
	close(conns)
	for c := range conns {
		defer c.Close(websocket.StatusNormalClosure, "")
	}

	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		if len(h.Rooms()) == 1 {
			break
		}
		time.Sleep(5 * time.Millisecond)
	}
	if rooms := h.Rooms(); len(rooms) != 1 || rooms[0] != "race" {
		t.Fatalf("Rooms=%v, want [race]", rooms)
	}
}
