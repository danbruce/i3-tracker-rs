extern crate chrono;
extern crate csv;
extern crate i3ipc;
extern crate xcb;

mod time_tracker;

fn main() {
    match time_tracker::track_time("output.log") {
        Err(e) => panic!("{:?}", e),
        _ => {}
    };
}
