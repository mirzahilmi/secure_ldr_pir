package config

type Config struct {
	Port            uint32
	IsDevelopment   bool
	ShutdownTimeout int64
	Oidc            Oidc
	Mqtt            Mqtt
}

type Oidc struct {
	Issuer   string
	ClientId string
}

type Mqtt struct {
	BrokerUrl,
	Username,
	Password,
	ClientId string
}
