package config

type Config struct {
	ShutdownTimeout int64
	Mqtt            Mqtt
	Otlp            Otlp
}

type Mqtt struct {
	BrokerUrl,
	ClientId string
}

type Otlp struct {
	CollectorEndpoint string
}
