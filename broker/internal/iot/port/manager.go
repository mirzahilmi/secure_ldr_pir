package port

import (
	"context"
	"sync"
)

type Hub struct {
	sync.RWMutex

	// Registered clients.
	clients map[*Client]bool

	// Inbound messages from the clients.
	broadcast chan []byte

	// Register requests from the clients.
	register chan *Client

	// Unregister requests from clients.
	unregister chan *Client
}

func (h *Hub) run(ctx context.Context) {
loop:
	for {
		select {
		case <-ctx.Done():
			break loop
		case client := <-h.register:
			func() {
				h.Lock()
				defer h.Unlock()
				h.clients[client] = true
			}()
		case client := <-h.unregister:
			func() {
				h.Lock()
				defer h.Unlock()
				if _, ok := h.clients[client]; ok {
					delete(h.clients, client)
					close(client.send)
				}
			}()
		case message := <-h.broadcast:
			for client := range h.clients {
				select {
				case client.send <- message:
				default:
					close(client.send)
					delete(h.clients, client)
				}
			}
		}
	}
}
