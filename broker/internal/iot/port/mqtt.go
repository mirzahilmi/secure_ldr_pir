package port

import (
	"bytes"
	"context"

	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/goccy/go-json"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/crypto"
	"github.com/rs/zerolog/log"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/metric"
)

type mqttHandler struct {
	// im not sure we supposed to put context here or if there's any better solution
	ctx      context.Context
	ldrGauge metric.Int64Gauge
	pirGauge metric.Int64Gauge
}

func NewMqttHandlers(ctx context.Context) ([]func() (string, byte, func(mqtt.Client, mqtt.Message)), error) {
	meter := otel.Meter("iot")

	ldrGauge, err := meter.Int64Gauge(
		"sensors.ldr",
		metric.WithDescription("LDR Sensor Reading"),
		metric.WithUnit("Ohms"),
	)
	if err != nil {
		log.Error().
			Err(err).
			Msg("iot: cannot create meter gauge instance")
		return nil, err
	}
	pirGauge, err := meter.Int64Gauge(
		"sensors.pir",
		metric.WithDescription("PIR Sensor Reading"),
		metric.WithUnit("Boolean"),
	)
	if err != nil {
		log.Error().
			Err(err).
			Msg("iot: cannot create meter gauge instance")
		return nil, err
	}

	h := mqttHandler{ctx, ldrGauge, pirGauge}

	handlers := []func() (string, byte, func(mqtt.Client, mqtt.Message)){
		func() (string, byte, func(mqtt.Client, mqtt.Message)) {
			return "esp32/kriptografi/encrypted/ldr-pir", 0, h.OnReading
		},
	}
	return handlers, nil
}

func (h *mqttHandler) OnReading(_ mqtt.Client, message mqtt.Message) {
	encryptedReading := new(EncryptedReading)
	if err := json.NewDecoder(bytes.NewBuffer(message.Payload())).Decode(encryptedReading); err != nil {
		log.Error().
			Err(err).
			Bytes("message", message.Payload()).
			Msg("iot: cannot decode mqtt message to reading struct")
		return
	}

	plaintext, err := crypto.EnvelopeUnseal(
		encryptedReading.Ciphertext,
		encryptedReading.Nonce,
		encryptedReading.Header.EphemeralPublicKey,
	)
	if err != nil {
		log.Error().
			Err(err).
			Bytes("message", message.Payload()).
			Msg("iot: cannot decode mqtt message to reading struct")
		return
	}

	var reading Reading
	if err := json.NewDecoder(bytes.NewBufferString(plaintext)).Decode(&reading); err != nil {
		log.Error().
			Err(err).
			Bytes("message", message.Payload()).
			Msg("iot: cannot decode mqtt message to reading struct")
		return
	}
	h.ldrGauge.Record(h.ctx, int64(reading.Ldr))
	h.pirGauge.Record(h.ctx, int64(func() int64 {
		if reading.Pir {
			return 1
		}
		return 0
	}()))

	log.Info().Str("data", plaintext).Msg("iot: unencrypted received message")
}
