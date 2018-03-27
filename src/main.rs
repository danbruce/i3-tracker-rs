extern crate chrono;
extern crate csv;
extern crate i3ipc;
extern crate fs2;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate xcb;

use std::time::Duration;

mod tick;
mod time_tracker;
mod track_i3;

fn main() {
    if let Err(e) = time_tracker::run("output.log", Duration::from_secs(10)) {
        panic!("{:?}", e);
    };
}
