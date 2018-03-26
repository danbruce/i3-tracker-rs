use super::time_tracker::LogEvent::I3Event;
use chrono::prelude::*;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::event::inner::WindowChange;
use std::error::Error;
use std::sync::mpsc::Sender;
use xcb;

#[derive(Clone)]
pub struct I3LogEvent {
    pub start_time: DateTime<Local>,
    pub window_id: u32,
    pub window_class: String,
    pub window_title: String,
}

impl I3LogEvent {
    fn new(window_id: u32, window_class: String, window_title: String) -> Self {
        I3LogEvent {
            start_time: Local::now(),
            window_id,
            window_class,
            window_title,
        }
    }
}

/*
 * Mostly pulled from:
 * https://stackoverflow.com/questions/44833160/how-do-i-get-the-x-window-class-given-a-window-id-with-rust-xcb
 */
fn get_class(conn: &xcb::Connection, window_id: u32) -> String {
    let long_length: u32 = 8;
    let mut long_offset: u32 = 0;
    let mut buf = Vec::new();
    loop {
        let cookie = xcb::xproto::get_property(
            conn,
            false,
            window_id,
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

pub fn run(sender: Sender<super::time_tracker::LogEvent>) -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let (xorg_conn, _) = xcb::Connection::connect(None)?;

    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    let mut prev_new_window_id: Option<i32> = None;
    for event in i3_listener.listen() {
        if let Event::WindowEvent(e) = event? {
            let window_id = e.container.window.unwrap_or(-1);
            if window_id < 1 {
                continue;
            }
            // new window events get duplicated in the listen loop so we need
            // to track a "new" event and ensure we only actually emit the title
            // event
            match e.change {
                WindowChange::New => {
                    prev_new_window_id = Some(window_id);
                    continue;
                }
                WindowChange::Focus => {
                    if let Some(prev_window_id) = prev_new_window_id {
                        if prev_window_id == window_id {
                            prev_new_window_id = None;
                            continue;
                        }
                    }
                }
                _ => {}
            };
            prev_new_window_id = None;
            match e.change {
                WindowChange::Focus | WindowChange::Title => {
                    let window_class = get_class(&xorg_conn, window_id as u32);
                    let window_title = e.container
                        .name
                        .clone()
                        .unwrap_or_else(|| "Untitled".into());
                    let send_event = I3Event(I3LogEvent::new(
                        window_id as u32,
                        window_class,
                        window_title,
                    ));
                    sender.send(send_event)?;
                }
                _ => {}
            };
        }
    }
    Ok(())
}
