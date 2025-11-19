package port

import (
	"context"
	"net/http"

	"github.com/danielgtaylor/huma/v2"
	"github.com/go-chi/chi/v5"
	"github.com/mirzahilmi/go-fast/internal/common/middleware"
	"github.com/rs/zerolog/log"
)

type handler struct {
	hub *Hub
}

func RegisterHandler(
	ctx context.Context,
	humaRouter huma.API,
	router chi.Router,
	middleware middleware.Middleware,
) {
	h := handler{hub: &Hub{
		broadcast:  make(chan []byte),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		clients:    make(map[*Client]bool),
	}}
	go h.hub.run(ctx)

	router.Get("/iot/readings", h.Connect(ctx))
}

func (h handler) Connect(ctx context.Context) func(w http.ResponseWriter, r *http.Request) {
	return func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Error().Err(err).Msg("")
			return
		}
		client := &Client{hub: h.hub, conn: conn, send: make(chan []byte, 256)}
		client.hub.register <- client

		go client.readPump(ctx)
		go client.writePump(ctx)
	}
}
