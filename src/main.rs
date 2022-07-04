extern crate chrono;
extern crate csv;
extern crate fs2;
extern crate i3ipc;
extern crate serde;

use std::time::Duration;

mod tick;
mod time_tracker;
mod track_i3;

fn main() {
    if let Err(e) = time_tracker::run("output.log", Duration::from_secs(10)) {
        panic!("{:?}", e);
    };
}
