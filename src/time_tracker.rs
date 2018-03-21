use chrono::prelude::*;
use csv::{Reader, Writer, WriterBuilder};
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::path::Path;
use xcb;

struct LogEvent {
    id: u32,
    start_time: DateTime<Local>,
    window_id: i32,
    window_class: String,
    window_title: String,
}

impl LogEvent {
    fn new(id: u32, event: &WindowEventInfo, xorg_conn: &xcb::Connection) -> LogEvent {
        let window_id = event.container.window.unwrap_or(-1);

        LogEvent {
            id,
            start_time: Local::now(),
            window_id,
            window_class: LogEvent::get_class(xorg_conn, window_id),
            window_title: event
                .container
                .name
                .clone()
                .unwrap_or_else(|| "Untitled".into()),
        }
    }
    /*
     * pulled from:
     * https://stackoverflow.com/questions/44833160/how-do-i-get-the-x-window-class-given-a-window-id-with-rust-xcb
     */
    fn get_class(conn: &xcb::Connection, id: i32) -> String {
        let window: xcb::xproto::Window = id as u32;
        let long_length: u32 = 8;
        let mut long_offset: u32 = 0;
        let mut buf = Vec::new();
        loop {
            let cookie = xcb::xproto::get_property(
                conn,
                false,
                window,
                xcb::xproto::ATOM_WM_CLASS,
                xcb::xproto::ATOM_STRING,
                long_offset,
                long_length,
            );
            match cookie.get_reply() {
                Ok(reply) => {
                    let value: &[u8] = reply.value();
                    buf.extend_from_slice(value);
                    match reply.bytes_after() {
                        0 => break,
                        _ => {
                            let len = reply.value_len();
                            long_offset += len / 4;
                        }
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
        let result = String::from_utf8(buf).unwrap();
        let results: Vec<_> = result.split('\0').collect();
        results[0].to_string()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Log {
    id: u32,
    start_time: String,
    end_time: String,
    duration: i64,
    window_id: i32,
    window_class: String,
    window_title: String,
}

impl Log {
    fn new(event: LogEvent) -> Log {
        let now = Local::now();
        let elapsed = now.signed_duration_since(event.start_time);
        Log {
            id: event.id,
            window_id: event.window_id,
            window_class: event.window_class,
            window_title: event.window_title,
            duration: elapsed.num_seconds(),
            start_time: event.start_time.format("%F %T").to_string(),
            end_time: now.format("%F %T").to_string(),
        }
    }
    fn write(&self, writer: &mut Writer<File>) -> Result<(), Box<Error>> {
        writer.serialize(self)?;
        writer.flush()?;
        Ok(())
    }
}

fn next_event_id<P: AsRef<Path>>(path: P) -> Result<u32, Box<Error>> {
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
    let wtr = WriterBuilder::new()
        .has_headers(has_headers)
        .from_writer(file);
    Ok(wtr)
}

pub fn run<P: AsRef<Path>>(out_path: P) -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let (xorg_conn, _) = xcb::Connection::connect(None)?;
    let mut next_event_id = next_event_id(&out_path)?;
    let mut writer = csv_writer(&out_path)?;

    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    let mut current_event: Option<LogEvent> = None;
    let mut last_event_new: bool = false;
    for event in i3_listener.listen() {
        if let Event::WindowEvent(e) = event? {
            match e.change {
                WindowChange::New => {
                    last_event_new = true;
                }
                WindowChange::Focus => {
                    if last_event_new {
                        last_event_new = false;
                        continue;
                    }
                    if let Some(e) = current_event {
                        Log::new(e).write(&mut writer)?;
                    }
                    current_event = Some(LogEvent::new(next_event_id, &e, &xorg_conn));
                    next_event_id += 1;
                }
                WindowChange::Title => {
                    last_event_new = false;
                    if let Some(e) = current_event {
                        Log::new(e).write(&mut writer)?;
                    }
                    current_event = Some(LogEvent::new(next_event_id, &e, &xorg_conn));
                    next_event_id += 1
                }
                _ => {}
            };
        }
    }
    Ok(())
}
