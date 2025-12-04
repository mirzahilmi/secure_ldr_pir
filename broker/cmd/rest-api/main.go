package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"os"
	"time"

	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/config"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/logging"
	"github.com/rs/zerolog/log"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/exporters/otlp/otlpmetric/otlpmetricgrpc"
	"go.opentelemetry.io/otel/sdk/metric"
	"go.opentelemetry.io/otel/sdk/resource"
	semconv "go.opentelemetry.io/otel/semconv/v1.37.0"
)

var cfg config.Config

func main() {
	logLevel := os.Getenv("LOG_LEVEL")
	configPath := os.Getenv("CONFIG_PATH")

	if logLevel == "" {
		log.Fatal().Msg("broker: missing LOG_LEVEL")
	}
	if configPath == "" {
		log.Fatal().Msg("broker: missing CONFIG_PATH")
	}

	logging.Init(logLevel)

	configBytes, err := os.ReadFile(configPath)
	if err != nil {
		log.Fatal().Err(err).Msg(fmt.Sprintf("broker: cannot read file %s", configPath))
	}
	if err := json.NewDecoder(bytes.NewBuffer(configBytes)).Decode(&cfg); err != nil {
		log.Fatal().Err(err).Msg("broker: failed to parse config raw bytes to struct")
	}
	ctx, mainCancel := context.WithCancel(context.Background())
	defer mainCancel()

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

	exporter, err := otlpmetricgrpc.New(
		ctx,
		otlpmetricgrpc.WithEndpoint(cfg.Otlp.CollectorEndpoint),
		otlpmetricgrpc.WithInsecure(),
	)
	if err != nil {
		log.Fatal().Err(err).Msg("broker: failed to setup otlp metric exporter")
	}
	defer exporter.Shutdown(ctx)

	meter := metric.NewMeterProvider(
		metric.WithResource(otlpRes),
		metric.WithReader(metric.NewPeriodicReader(exporter)),
	)
	otel.SetMeterProvider(meter)

	mqttOpts := mqtt.NewClientOptions().
		AddBroker(cfg.Mqtt.BrokerUrl).
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

	if err := setup(ctx, mqttOpts); err != nil {
		log.Fatal().Err(err).Msg("broker: failed to setup app")
	}

	mqttClient := mqtt.NewClient(mqttOpts)
	defer mqttClient.Disconnect(uint(cfg.ShutdownTimeout))

	for {
		log.Info().Msg(fmt.Sprintf("broker: listening mqtt on broker %s", cfg.Mqtt.BrokerUrl))
		if token := mqttClient.Connect(); token.Wait() && token.Error() != nil {
			log.Fatal().Err(err).Msg("broker: failed connecting to mqtt broker")
			time.Sleep(2 * time.Second)
			continue
		}
		break
	}
	<-ctx.Done()
}
