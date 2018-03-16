extern crate i3ipc;
extern crate xcb;

mod time_tracker;

fn main() {
    time_tracker::track_time();
}
