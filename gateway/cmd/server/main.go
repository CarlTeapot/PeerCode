package main

// Entry point. Wires everything together:
//   - reads config (port, etc.)
//   - creates the Hub
//   - starts the HTTP server with chi router
//   - mounts WebSocket handler at /ws
//   - mounts health-check at /health
//
// Nothing else lives here.

func main() {}
