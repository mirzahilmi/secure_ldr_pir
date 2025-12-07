use anyhow::Result;
use esp_idf_svc::hal::gpio::{
    PinDriver, 
    Input,
    Gpio23,
};

pub struct Sensors {
    pub pir: PinDriver<'static, Gpio23, Input>,
}

impl Sensors {
    pub fn new(gpio23: Gpio23) -> Result<Self> {
        let pir = PinDriver::input(gpio23)?;

        Ok(Self { pir })
    }
}
