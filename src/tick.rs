use super::time_tracker::LogEvent;
use std::error::Error;
use std::sync::mpsc::Sender;
use std::thread::sleep;
use std::time::Duration;

pub struct TickEvent(pub u32);

pub fn run(
    sender: Sender<super::time_tracker::LogEvent>,
    event_id: u32,
    sleep_for: Duration,
) -> Result<(), Box<dyn Error>> {
    sleep(sleep_for);
    sender.send(LogEvent::TickEvent(TickEvent(event_id)))?;
    Ok(())
}
