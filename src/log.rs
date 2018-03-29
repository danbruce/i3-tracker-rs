use csv::Writer;
use std::error::Error;
use std::fs::File;

#[derive(Debug, Deserialize, Serialize)]
pub struct Log {
    pub id: u32,
    pub start_time: String,
    pub end_time: String,
    pub duration: i64,
    pub window_id: u32,
    pub window_class: String,
    pub window_title: String,
}

impl Log {
    pub fn write(&self, writer: &mut Writer<File>) -> Result<(), Box<Error>> {
        writer.serialize(self)?;
        writer.flush()?;
        Ok(())
    }
}

pub trait ToLog {
    fn to_log(&self, event_id: u32) -> Log;
}
