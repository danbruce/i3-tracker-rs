use csv::{Reader, Writer};
use std::{error::Error, fs::{File, OpenOptions}, path::Path};

#[derive(Debug, Deserialize, Serialize)]
pub struct LogRow {
    pub id: u32,
    pub start_time: String,
    pub end_time: String,
    pub duration: i64,
    pub window_id: u32,
    pub window_class: String,
    pub window_title: String,
}

impl LogRow {
    pub fn write(&self, writer: &mut Writer<File>) -> Result<(), Box<Error>> {
        writer.serialize(self)?;
        writer.flush()?;
        Ok(())
    }
    pub fn read<P: AsRef<Path>>(path: P) -> Result<LogRow, Box<Error>> {
        if let Ok(f) = OpenOptions::new().read(true).open(path) {
            let mut r = Reader::from_reader(f);
            if let Some(res) = r.deserialize().last() {
                return Ok(res?);
            }
        }
        Err(String::from("file not found"))?
    }
}

pub trait Log: Send {
    fn to_log(&self, event_id: u32) -> LogRow;
}
