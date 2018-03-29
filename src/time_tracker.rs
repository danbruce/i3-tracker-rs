use csv::{Reader, Writer, WriterBuilder};
use fs2::FileExt;
use log::{LogRow, Log};
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use track_i3;

fn initial_event_id<P: AsRef<Path>>(path: P) -> Result<u32, Box<Error>> {
    if let Ok(f) = OpenOptions::new().read(true).open(path) {
        let mut r = Reader::from_reader(f);
        if let Some(res) = r.deserialize().last() {
            let log: LogRow = res?;
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

pub fn run<P: AsRef<Path>>(out_path: P, tick_sleep: Duration) -> Result<(), Box<Error>> {
    let (tx, rx) = channel();
    // start the i3 event listening thread
    {
        let tx = tx.clone();
        thread::spawn(move || {
            track_i3::run(tx, tick_sleep).unwrap();
        });
    }

    let mut next_event_id = initial_event_id(&out_path)?;
    let mut previous_event: Option<Box<Log>> = None;
    let mut writer = csv_writer(&out_path)?;
    loop {
        let event = rx.recv()?;
        if let Some(prev) = previous_event {
            let log = prev.to_log(next_event_id);
            log.write(&mut writer)?;
            next_event_id += 1;
        }
        previous_event = Some(event);
    }
}
