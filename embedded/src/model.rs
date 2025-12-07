use serde::Serialize;

#[derive(Serialize)]
pub struct Telemetry {
    pub device_id: &'static str,
    pub timestamp_ms: u64,
    pub ldr: u16,
    pub pir: bool,
}