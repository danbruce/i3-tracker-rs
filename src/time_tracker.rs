use chrono::prelude::*;
use csv::{Reader, Writer};
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::event::inner::WindowChange;
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
            Err(err) => {
                println!("{:?}", err);
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

pub fn track_time(output_filename: &str) -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let (xorg_conn, _screen_num) = xcb::Connection::connect(None)?;
    let mut first_event_id = 1;
    let mut writer = match OpenOptions::new().append(true).open(output_filename) {
        Ok(f) => {
            let mut r =
                Reader::from_reader(OpenOptions::new().read(true).open(output_filename).unwrap());
            match r.into_records().last() {
                Some(last_line) => {
                    match last_line {
                        Ok(r) => {
                            if r.len() > 0 {
                                match r[0].parse::<u32>() {
                                    Ok(i) => {
                                        first_event_id = i + 1;
                                    }
                                    Err(_) => {}
                                }
                            }
                        }
                        Err(_) => {}
                    };
                }
                None => {}
            };
            Writer::from_writer(f)
        }
        Err(_) => match OpenOptions::new()
            .create(true)
            .write(true)
            .open(output_filename)
        {
            Ok(f) => {
                let mut w = Writer::from_writer(f);
                write_header_to_file(&mut w).unwrap();
                w
            }
            Err(e) => {
                panic!("Unable to open log file: {}", e);
            }
        },
    };

    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    let mut current_event: Option<LogEvent> = None;
    for event in i3_listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                match &current_event {
                    &Some(ref e) => {
                        write_event_to_file(&mut writer, &e)?;
                    }
                    &None => {}
                };
                match e.change {
                    WindowChange::Focus | WindowChange::Title => {
                        if let &Some(ref window) = &e.container.window {
                            current_event = Some(LogEvent {
                                id: match current_event {
                                    Some(e) => e.id + 1,
                                    None => first_event_id,
                                },
                                start_date_time: Local::now(),
                                window_id: *window as usize,
                                window_class: get_class(&xorg_conn, &window),
                                window_title: e.container.name.unwrap_or("Untitled".to_string()),
                            });
                        }
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
    window_id: usize,
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
