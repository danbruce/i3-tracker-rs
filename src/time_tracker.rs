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

/*
 * pulled from:
 * https://stackoverflow.com/questions/44833160/how-do-i-get-the-x-window-class-given-a-window-id-with-rust-xcb
 */
fn get_class(conn: &xcb::Connection, id: &i32) -> String {
    let window: xcb::xproto::Window = *id as u32;
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
    //         String::from_utf8(buf)
    //         .unwrap()
    //         .split('\0')
    //         .take(1)
    //         .next()
    //         .unwrap()
    //         .to_string()
}

fn write_event_to_file(writer: &mut Writer<File>, log: &Log) -> Result<(), Box<Error>> {
    writer.serialize(log)?;
    writer.flush()?;
    Ok(())
}

fn next_event(
    next_event_id: &mut u32,
    event: &WindowEventInfo,
    xorg_conn: &xcb::Connection,
) -> LogEvent {
    let window_id: i32 = match event.container.window {
        Some(w) => w,
        None => -1,
    };
    *next_event_id += 1;

    LogEvent {
        id: *next_event_id - 1,
        start_time: Local::now(),
        window_id,
        window_class: get_class(&xorg_conn, &window_id),
        window_title: event.container.name.clone().unwrap_or("Untitled".into()),
    }
}

fn next_event_id<P: AsRef<Path>>(output_filename: P) -> Result<u32, Box<Error>> {
    if let Ok(f) = OpenOptions::new().read(true).open(output_filename) {
        let mut r = Reader::from_reader(f);
        if let Some(res) = r.deserialize().last() {
            let log: Log = res?;
            return Ok(log.id);
        }
    }
    Ok(1)
}

fn csv_writer<P: AsRef<Path>>(path: P) -> Result<Writer<File>, Box<Error>> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())?;
    let wtr = WriterBuilder::new()
        .has_headers(!Path::new(path.as_ref()).exists())
        .from_writer(file);
    Ok(wtr)
}

pub fn track_time<P: AsRef<Path>>(out_path: P) -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let (xorg_conn, _screen_num) = xcb::Connection::connect(None)?;
    let mut next_event_id = next_event_id(&out_path)?;
    let mut writer = csv_writer(&out_path)?;

    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    let mut current_event: Option<LogEvent> = None;
    let mut last_event_new: bool = false;
    for event in i3_listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
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
                            write_event_to_file(&mut writer, &Log::new(e))?;
                        }
                        current_event = Some(next_event(&mut next_event_id, &e, &xorg_conn));
                    }
                    WindowChange::Title => {
                        last_event_new = false;
                        if let Some(e) = current_event {
                            write_event_to_file(&mut writer, &Log::new(e))?;
                        }
                        current_event = Some(next_event(&mut next_event_id, &e, &xorg_conn));
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    }
    Ok(())
}

struct LogEvent {
    id: u32,
    start_time: DateTime<Local>,
    window_id: i32,
    window_class: String,
    window_title: String,
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
}
