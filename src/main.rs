extern crate i3ipc;
extern crate xcb;

mod time_tracker;

fn main() {
    let mut tracker = time_tracker::TimeTracker::new();
    tracker.run();
}
