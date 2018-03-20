extern crate chrono;
extern crate csv;
extern crate i3ipc;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate xcb;

mod time_tracker;

fn main() {
    match time_tracker::track_time("output.log") {
        Err(e) => panic!("{:?}", e),
        _ => {}
    };
}
