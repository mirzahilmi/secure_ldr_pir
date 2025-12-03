package main

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"os"
	"time"

	"github.com/danielgtaylor/huma/v2"
	"github.com/danielgtaylor/huma/v2/adapters/humachi"
	"github.com/danielgtaylor/huma/v2/humacli"
	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/go-chi/chi/v5"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/config"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/constant"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/logging"
	"github.com/rs/zerolog/log"
	"github.com/spf13/cobra"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/exporters/otlp/otlpmetric/otlpmetricgrpc"
	"go.opentelemetry.io/otel/sdk/metric"
	"go.opentelemetry.io/otel/sdk/resource"
	semconv "go.opentelemetry.io/otel/semconv/v1.37.0"
)

type options struct {
	LogLevel   string `doc:"Log verbosity level" default:"info"`
	ConfigPath string `doc:"Configuration path [REQUIRED]" name:"config"`
}

var (
	api    huma.API
	router *chi.Mux
	cfg    config.Config
)

func main() {
	cli := humacli.New(func(hooks humacli.Hooks, options *options) {
		logging.Init(options.LogLevel)

		if options.ConfigPath == "" {
			log.Fatal().Msg("broker: missing CONFIG_PATH")
		}
		configBytes, err := os.ReadFile(options.ConfigPath)
		if err != nil {
			log.Fatal().Err(err).Msg(fmt.Sprintf("broker: cannot read file %s", options.ConfigPath))
		}
		if err := json.NewDecoder(bytes.NewBuffer(configBytes)).Decode(&cfg); err != nil {
			log.Fatal().Err(err).Msg("broker: failed to parse config raw bytes to struct")
		}
		ctx, mainCancel := context.WithCancel(context.Background())

		oapi := huma.DefaultConfig("Secured LDR PIR Sensor Broker - OpenAPI 3.0", "1.0.0")
		oapi.DocsPath = ""
		oapi.Info.Description = constant.OAPI_SPEC_DESCRIPTION
		oapi.Components.SecuritySchemes = map[string]*huma.SecurityScheme{
			constant.OAPI_SECURITY_SCHEME: {
				Type: "oauth2",
				Flows: &huma.OAuthFlows{
					Implicit: &huma.OAuthFlow{
						AuthorizationURL: fmt.Sprintf(
							"%s/protocol/openid-connect/auth",
							cfg.Oidc.Issuer,
						),
						Scopes: map[string]string{
							"openid":  "openid",
							"profile": "profile",
						},
					},
				},
			},
		}
		oapi.Security = []map[string][]string{
			{constant.OAPI_SECURITY_SCHEME: {}},
		}

		otlpRes, err := resource.Merge(
			resource.Default(),
			resource.NewWithAttributes(
				semconv.SchemaURL,
				semconv.ServiceName("broker"),
				semconv.ServiceVersion("0.1.0"),
			),
		)
		if err != nil {
			log.Fatal().Err(err).Msg("broker: failed to setup otlp resource")
		}

		exporter, err := otlpmetricgrpc.New(ctx)
		if err != nil {
			log.Fatal().Err(err).Msg("broker: failed to setup otlp metric exporter")
		}
		meter := metric.NewMeterProvider(
			metric.WithResource(otlpRes),
			metric.WithReader(metric.NewPeriodicReader(exporter)),
		)
		otel.SetMeterProvider(meter)

		router = chi.NewRouter()
		if cfg.IsDevelopment {
			router.Get("/docs", func(w http.ResponseWriter, r *http.Request) {
				w.Header().Set("Content-Type", "text/html")
				if _, err := w.Write([]byte(constant.OAPI_SPEC_UI)); err != nil {
					log.Debug().Err(err).Msg("broker: failed to write openapi editor ui")
				}
			})
		}

		mqttOpts := mqtt.NewClientOptions().
			AddBroker(cfg.Mqtt.BrokerUrl).
			SetUsername(cfg.Mqtt.Username).
			SetPassword(cfg.Mqtt.Password).
			SetClientID(cfg.Mqtt.ClientId).
			SetCleanSession(true).
			SetAutoReconnect(true).
			SetConnectRetry(true).
			SetConnectRetryInterval(3 * time.Second).
			SetKeepAlive(10 * time.Second).
			SetPingTimeout(5 * time.Second).
			SetWriteTimeout(10 * time.Second).
			SetDefaultPublishHandler(func(_ mqtt.Client, message mqtt.Message) {
				log.Warn().Bytes("data", message.Payload()).Msg("broker: mqtt fallback handling")
			}).
			SetConnectionLostHandler(func(_ mqtt.Client, err error) {
				log.Warn().Err(err).Msg("broker: mqtt connection lost")
			})

		api = humachi.New(router, oapi)
		if err := setup(ctx, mqttOpts); err != nil {
			log.Fatal().Err(err).Msg("broker: failed to setup")
		}
		mqttClient := mqtt.NewClient(mqttOpts)

		addr := fmt.Sprintf(":%d", cfg.Port)
		server := http.Server{
			Addr:              addr,
			Handler:           router,
			ReadHeaderTimeout: 5 * time.Second, // mitigate slowloris attacks
		}

		hooks.OnStart(func() {
			go func() {
				for {
					log.Info().Msg(fmt.Sprintf("broker: listening mqtt on broker %s", cfg.Mqtt.BrokerUrl))
					if token := mqttClient.Connect(); token.Wait() && token.Error() != nil {
						log.Fatal().Err(err).Msg("broker: failed connecting to mqtt broker")
						time.Sleep(2 * time.Second)
						continue
					}
					break
				}
			}()

			log.Info().Msg(fmt.Sprintf("broker: listening on 0.0.0.0%s", addr))
			if err := server.ListenAndServe(); !errors.Is(err, http.ErrServerClosed) {
				log.Fatal().Err(err).Msg(fmt.Sprintf("broker: failed to listen on 0.0.0.0%s", addr))
			}
		})

		hooks.OnStop(func() {
			ctx, cancel := context.WithTimeout(
				context.Background(),
				time.Duration(cfg.ShutdownTimeout)*time.Second,
			)
			defer cancel()

			if err := server.Shutdown(ctx); err != nil {
				log.Fatal().Err(err).Msg("broker: failed to shutdown")
			}
			mainCancel()
			log.Info().Msg("broker: shut down complete")

			mqttClient.Disconnect(uint(cfg.ShutdownTimeout))
		})
	})

	cli.Root().AddCommand(&cobra.Command{
		Use:   "spec",
		Short: "Print the OpenAPI specification",
		RunE: func(cmd *cobra.Command, args []string) error {
			var spec []byte
			if len(args) == 1 && args[0] == "legacy" {
				raw, err := api.OpenAPI().DowngradeYAML()
				if err != nil {
					return err
				}
				spec = raw
			} else {
				raw, err := api.OpenAPI().YAML()
				if err != nil {
					return err
				}
				spec = raw
			}
			fmt.Println(string(spec))

			return nil
		},
	})

	cli.Run()
}
