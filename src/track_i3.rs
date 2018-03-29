use chrono::prelude::*;
use i3ipc;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::inner::WindowChange;
use log::{Log, ToLog};
use std::error::Error;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use xcb;

#[derive(Clone)]
struct I3LogEvent {
    start_time: DateTime<Local>,
    window_id: u32,
    window_class: String,
    window_title: String,
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
    fn from_prev(old_event: &Self) -> Self {
        I3LogEvent {
            start_time: Local::now(),
            window_id: old_event.window_id,
            window_class: old_event.window_class.clone(),
            window_title: old_event.window_title.clone(),
        }
    }
}

impl ToLog for I3LogEvent {
    fn to_log(&self, event_id: u32) -> Log {
        let now = Local::now();
        let elapsed = now.signed_duration_since(self.start_time);
        Log {
            id: event_id,
            window_id: self.window_id,
            window_class: self.window_class.clone(),
            window_title: self.window_title.clone(),
            duration: elapsed.num_seconds(),
            start_time: self.start_time.format("%F %T").to_string(),
            end_time: now.format("%F %T").to_string(),
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

enum Event {
    I3(i3ipc::event::Event),
    Tick(u32),
}

fn timeout(sender: Sender<Event>, event_id: u32, sleep_for: Duration) -> Result<(), Box<Error>> {
    sleep(sleep_for);
    sender.send(Event::Tick(event_id))?;
    Ok(())
}

fn capture_i3_events(sender: Sender<Event>) -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    for event in i3_listener.listen() {
        if let Ok(e) = event {
            sender.send(Event::I3(e))?;
        }
    }
    Ok(())
}

pub fn run(sender: Sender<Box<ToLog + Send>>, sleep_for: Duration) -> Result<(), Box<Error>> {
    let (tx, rx) = channel();
    {
        let tx = tx.clone();
        thread::spawn(move || {
            capture_i3_events(tx).unwrap();
        });
    }
    let (xorg_conn, _) = xcb::Connection::connect(None)?;
    let mut prev_new_window_id: Option<i32> = None;
    let mut i3_event_id_sequence = 1;
    let mut prev_i3_event: Option<I3LogEvent> = None;
    loop {
        let event = rx.recv()?;
        match event {
            Event::I3(e) => {
                if let i3ipc::event::Event::WindowEvent(e) = e {
                    let window_id = e.container.window.unwrap_or(-1);
                    if window_id < 1 {
                        continue;
                    }
                    match e.change {
                        WindowChange::New => {
                            prev_new_window_id = Some(window_id);
                            continue;
                        }
                        WindowChange::Title => {
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
                            let new_event =
                                I3LogEvent::new(window_id as u32, window_class, window_title);
                            {
                                let new_event = new_event.clone();
                                prev_i3_event = Some(new_event);
                            }
                            sender.send(Box::new(new_event))?;
                        }
                        _ => {}
                    };
                }
            }
            Event::Tick(id) => {
                if id != i3_event_id_sequence {
                    continue;
                }
                if let Some(prev) = prev_i3_event {
                    let new_event = I3LogEvent::from_prev(&prev);
                    {
                        let new_event = new_event.clone();
                        prev_i3_event = Some(new_event);
                    }
                    sender.send(Box::new(new_event))?;
                    i3_event_id_sequence += 1;
                    {
                        let tx = tx.clone();
                        thread::spawn(move || {
                            timeout(tx, i3_event_id_sequence, sleep_for).unwrap();
                        });
                    }
                }
            }
        };
    }
}
