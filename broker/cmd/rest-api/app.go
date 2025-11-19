package main

import (
	"context"

	"github.com/mirzahilmi/go-fast/internal/common/middleware"
	iot "github.com/mirzahilmi/go-fast/internal/iot/port"
	"github.com/mirzahilmi/go-fast/internal/utility"
)

func setup(ctx context.Context) error {
	middleware := middleware.NewMiddleware(api, cfg)

	utility.RegisterHandler(ctx, api, middleware)
	iot.RegisterHandler(ctx, api, router, middleware)

	return nil
}
