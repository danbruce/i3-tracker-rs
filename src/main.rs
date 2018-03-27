extern crate chrono;
extern crate csv;
extern crate i3ipc;
extern crate fs2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate xcb;

mod time_tracker;
mod track_i3;

fn main() {
    if let Err(e) = time_tracker::run("output.log") {
        panic!("{:?}", e);
    };
}
