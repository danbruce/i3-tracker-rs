use chrono::prelude::*;
use csv::{Reader, Writer};
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::event::inner::WindowChange;
use i3ipc::event::WindowEventInfo;
use std::error::Error;
use std::fs::{File, OpenOptions};
use xcb;

/**
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
    let results: Vec<&str> = result.split('\0').collect();
    results[0].to_string()
}

fn write_event_to_file(writer: &mut Writer<File>, e: &LogEvent) -> Result<(), Box<Error>> {
    let row = e.get_output_row()?;
    writer.write_record(&row)?;
    writer.flush()?;
    Ok(())
}
fn write_header_to_file(writer: &mut Writer<File>) -> Result<(), Box<Error>> {
    let header: OutputRow = [
        "id".to_string(),
        "window_id".to_string(),
        "window_title".to_string(),
        "window_class".to_string(),
        "start_time".to_string(),
        "end_time".to_string(),
        "duration".to_string(),
    ];
    writer.write_record(&header)?;
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
    let window_title: String = match &event.container.name {
        &Some(ref s) => s.clone(),
        &None => "Untitled".to_string(),
    };
    *next_event_id += 1;
    LogEvent {
        id: *next_event_id - 1,
        start_date_time: Local::now(),
        window_id: window_id,
        window_class: get_class(&xorg_conn, &window_id),
        window_title: window_title,
    }
}

fn get_next_event_id(output_filename: &str) -> u32 {
    if let Ok(f) = OpenOptions::new().read(true).open(output_filename) {
        let mut r = Reader::from_reader(f);
        if let Some(Ok(last_line)) = r.into_records().last() {
            if last_line.len() > 0 {
                if let Ok(i) = last_line[0].parse::<u32>() {
                    return i;
                }
            }
        }
    }
    1
}

fn get_csv_writer(output_filename: &str, has_lines: bool) -> Result<Writer<File>, Box<Error>> {
    if let Ok(append) = OpenOptions::new().append(true).open(output_filename) {
        if has_lines {
            return Ok(Writer::from_writer(append));
        }
    }
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(output_filename)?;
    let mut writer = Writer::from_writer(file);
    write_header_to_file(&mut writer)?;
    Ok(writer)
}

pub fn track_time(output_filename: &str) -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let (xorg_conn, _screen_num) = xcb::Connection::connect(None)?;
    let mut next_event_id = get_next_event_id(output_filename);
    let mut writer = get_csv_writer(output_filename, next_event_id > 1)?;

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
                            write_event_to_file(&mut writer, &e)?;
                        }
                        current_event = Some(next_event(&mut next_event_id, &e, &xorg_conn));
                    }
                    WindowChange::Title => {
                        last_event_new = false;
                        if let Some(e) = current_event {
                            write_event_to_file(&mut writer, &e)?;
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

type OutputRow = [String; 7];

struct LogEvent {
    id: u32,
    start_date_time: DateTime<Local>,
    window_id: i32,
    window_class: String,
    window_title: String,
}

impl LogEvent {
    fn get_output_row(&self) -> Result<OutputRow, Box<Error>> {
        let now = Local::now();
        let elapsed = now.signed_duration_since(self.start_date_time);
        Ok([
            self.id.to_string(),
            self.window_id.to_string(),
            self.window_title.clone(),
            self.window_class.clone(),
            self.start_date_time.format("%F %T").to_string(),
            now.format("%F %T").to_string(),
            elapsed.num_seconds().to_string(),
        ])
    }
}
