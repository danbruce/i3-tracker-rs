use super::track_i3;
use chrono::prelude::*;
use csv::{Reader, Writer, WriterBuilder};
use fs2::FileExt;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

#[derive(Debug, Deserialize, Serialize)]
struct Log {
    id: u32,
    start_time: String,
    end_time: String,
    duration: i64,
    window_id: u32,
    window_class: String,
    window_title: String,
}

impl Log {
    fn new(id: u32, event: &LogEvent) -> Log {
        match event {
            &LogEvent::I3Event(ref e) => {
                let now = Local::now();
                let elapsed = now.signed_duration_since(e.start_time);
                Log {
                    id,
                    window_id: e.window_id,
                    window_class: e.window_class.clone(),
                    window_title: e.window_title.clone(),
                    duration: elapsed.num_seconds(),
                    start_time: e.start_time.format("%F %T").to_string(),
                    end_time: now.format("%F %T").to_string(),
                }
            }
        }
    }
    fn write(&self, writer: &mut Writer<File>) -> Result<(), Box<Error>> {
        writer.serialize(self)?;
        writer.flush()?;
        Ok(())
    }
}

fn initial_event_id<P: AsRef<Path>>(path: P) -> Result<u32, Box<Error>> {
    if let Ok(f) = OpenOptions::new().read(true).open(path) {
        let mut r = Reader::from_reader(f);
        if let Some(res) = r.deserialize().last() {
            let log: Log = res?;
            return Ok(log.id + 1);
        }
    }
    Ok(1)
}

fn csv_writer<P: AsRef<Path>>(path: P) -> Result<Writer<File>, Box<Error>> {
    let has_headers = !Path::new(path.as_ref()).exists();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())?;
    file.try_lock_exclusive()?;
    let wtr = WriterBuilder::new()
        .has_headers(has_headers)
        .from_writer(file);
    Ok(wtr)
}

// the possible events we receive off the channel
pub enum LogEvent {
    I3Event(track_i3::I3LogEvent),
}

pub fn run<P: AsRef<Path>>(out_path: P) -> Result<(), Box<Error>> {
    let (tx, rx): (Sender<LogEvent>, Receiver<LogEvent>) = mpsc::channel();
    // start the i3 event listening thread
    thread::spawn(move || {
        track_i3::run(tx.clone()).unwrap();
    });

    let mut next_event_id = initial_event_id(&out_path)?;
    let mut writer = csv_writer(&out_path)?;
    let mut prev_i3_event: Option<track_i3::I3LogEvent> = None;
    loop {
        let event = rx.recv()?;
        match &event {
            &LogEvent::I3Event(ref e) => {
                if let Some(prev) = prev_i3_event {
                    let log = Log::new(next_event_id, &LogEvent::I3Event(prev));
                    log.write(&mut writer)?;
                    next_event_id += 1;
                }
                prev_i3_event = Some(e.clone());
            }
        }
    }
}
