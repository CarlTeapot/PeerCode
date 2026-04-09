package main

import (
	"fmt"
	"net"
	"net/http"
	"os"

	"gateway/internal/hub"
)

func main() {
	ln, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		fmt.Fprintf(os.Stderr, "listen: %v\n", err)
		os.Exit(1)
	}

	port := ln.Addr().(*net.TCPAddr).Port

	fmt.Printf("PORT=%d\n", port)
	_ = os.Stdout.Sync()

	h := hub.New()

	mux := http.NewServeMux()
	mux.HandleFunc("/ws", h.HandleWS)
	mux.HandleFunc("/health", func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
	})

	if err := http.Serve(ln, mux); err != nil {
		fmt.Fprintf(os.Stderr, "serve: %v\n", err)
		os.Exit(1)
	}
}
