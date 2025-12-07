use std::thread;
use esp_idf_svc::sntp::{
    EspSntp,
    SyncStatus
};
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};
use chrono::{
    DateTime,
    Utc,
    FixedOffset,
};
use anyhow::Result;
use esp_idf_svc::hal::gpio::{
    self as gpio,
    PinDriver,
    Output,
};


pub fn sync_time_via_ntp(sntp_instance: &EspSntp, led: &mut PinDriver<'_, gpio::Gpio2, Output>) -> Result<()> {
    log::info!("Starting NTP time sync...");

    while sntp_instance.get_sync_status() != SyncStatus::Completed {
        log::info!("Waiting for NTP time sync...");

        for _ in 0..2 {
            led.set_high()?;
            thread::sleep(Duration::from_millis(50));
            led.set_low()?;
            thread::sleep(Duration::from_millis(50));
        }

        thread::sleep(Duration::from_millis(500));
    }

    led.set_low()?;
    log::info!("NTP time sync completed.");

    let system_time_now = SystemTime::now();
    let utc: DateTime<Utc> = system_time_now.into();

    let utc_plus_7 = utc.with_timezone(&FixedOffset::east_opt(7 * 3600).unwrap());

    log::info!("Current time (UTC+7): {}", utc_plus_7.format("%d/%m/%Y %H:%M:%S"));

    Ok(())
}

pub fn get_timestamp_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}