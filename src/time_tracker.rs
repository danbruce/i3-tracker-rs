use i3ipc::I3EventListener;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::event::inner::WindowChange;
use xcb;

pub struct TimeTracker {
    i3_listener: I3EventListener,
    xorg_conn: xcb::Connection,
}

impl TimeTracker {
    pub fn new() -> Self {
        let i3_listener = I3EventListener::connect().unwrap();
        let (xorg_conn, _screen_num) = xcb::Connection::connect(None).unwrap();
        TimeTracker {
            i3_listener,
            xorg_conn,
        }
    }
    pub fn run(&mut self) {
        let subs = [Subscription::Window];
        self.i3_listener.subscribe(&subs).unwrap();

        // handle them
        for event in self.i3_listener.listen() {
            match event.unwrap() {
                Event::WindowEvent(e) => match e.change {
                    WindowChange::Focus => match e.container.name {
                        Some(n) => println!(
                            "{} ({})",
                            n,
                            get_class(&self.xorg_conn, &e.container.window.unwrap())
                        ),
                        None => println!("Untitled ({})", get_class(&self.xorg_conn, &e.container.window.unwrap())),
                    },
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

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
            &conn,
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
