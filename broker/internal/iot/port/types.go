package port

type EncryptedReading struct {
	Header     Header `json:"header"`
	Ciphertext string `json:"ciphertext"`
	Nonce      string `json:"nonce"`
	Metric     Metric `json:"metrics"`
}

type Metric struct {
	EncryptTime uint64 `json:"encrypt_time_ns"`
}

type Header struct {
	Algorithm          string `json:"algorithm"`
	EphemeralPublicKey string `json:"ephemeral_public_key"`
}

type Reading struct {
	DeviceId    string `json:"device_id"`
	TimestampMs uint64 `json:"timestamp_ms"`
	Ldr         uint64 `json:"ldr"`
	Pir         bool   `json:"pir"`
}
