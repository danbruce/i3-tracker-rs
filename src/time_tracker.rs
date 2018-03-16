use csv::Writer;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::event::inner::WindowChange;
use std::error::Error;
use std::fs::File;
use std::time::Duration;
use std::time::SystemTime;
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

fn write_to_file(
    writer: &mut Writer<File>,
    e: &LogEvent,
    length: Duration,
) -> Result<(), Box<Error>> {
    writer.write_record(&[
        e.title.clone(),
        e.window_class.clone(),
        length.as_secs().to_string(),
    ])?;
    writer.flush()?;
    Ok(())
}

pub fn track_time() -> Result<(), Box<Error>> {
    let mut i3_listener = I3EventListener::connect()?;
    let (xorg_conn, _screen_num) = xcb::Connection::connect(None)?;
    let mut writer = Writer::from_path("output.log")?;

    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    let mut current_event: Option<LogEvent> = None;
    for event in i3_listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                match &current_event {
                    &Some(ref e) => {
                        let elapsed = e.time.elapsed()?;
                        write_to_file(&mut writer, &e, elapsed)?;
                    }
                    _ => {}
                };
                match e.change {
                    WindowChange::Focus => {
                        if let &Some(ref window) = &e.container.window {
                            current_event = Some(LogEvent {
                                time: SystemTime::now(),
                                title: e.container.name.unwrap_or("Untitled".to_string()),
                                window_class: get_class(&xorg_conn, &window),
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

struct LogEvent {
    time: SystemTime,
    title: String,
    window_class: String,
}
