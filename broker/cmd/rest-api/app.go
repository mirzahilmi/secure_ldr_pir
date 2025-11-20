package main

import (
	"context"

	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/middleware"
	iot "github.com/mirzahilmi/secure_ldr_pir/broker/internal/iot/port"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/utility"
)

func setup(ctx context.Context) error {
	middleware := middleware.NewMiddleware(api, cfg)

	utility.RegisterHandler(ctx, api, middleware)
	iot.RegisterHandler(ctx, api, router, middleware)

	return nil
}
