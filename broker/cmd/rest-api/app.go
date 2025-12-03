package main

import (
	"context"

	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/common/middleware"
	iot "github.com/mirzahilmi/secure_ldr_pir/broker/internal/iot/port"
	"github.com/mirzahilmi/secure_ldr_pir/broker/internal/utility"
	"github.com/rs/zerolog/log"
)

func setup(ctx context.Context, mqttOpts *mqtt.ClientOptions) error {
	middleware := middleware.NewMiddleware(api, cfg)

	utility.RegisterHandler(ctx, api, middleware)

	mqttHandlers := []func() (string, byte, func(mqtt.Client, mqtt.Message)){}
	iotMqttHandlers, err := iot.NewMqttHandlers(ctx)
	if err != nil {
		return err
	}

	mqttHandlers = append(mqttHandlers, iotMqttHandlers...)

	mqttOpts.SetOnConnectHandler(func(c mqtt.Client) {
		log.Debug().Msg("mqtt: connected")
		topics := []string{}
		for _, handle := range mqttHandlers {
			topic, qos, topicHandle := handle()
			if token := c.Subscribe(topic, qos, topicHandle); token.Wait() && token.Error() != nil {
				log.Error().Err(token.Error()).Msg("mqtt: failed to subscribe topic")
				continue
			}
			topics = append(topics, topic)
		}
		log.Info().Strs("topics", topics).Msg("mqtt: topics subscribed")
	})

	return nil
}
