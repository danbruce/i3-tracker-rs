extern crate chrono;
extern crate csv;
extern crate i3ipc;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate xcb;

mod time_tracker;
mod error;

fn main() {
    if let Err(e) = time_tracker::run("output.log") {
        panic!("{:?}", e);
    };
}
