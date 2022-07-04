use super::time_tracker::LogEvent::I3Event;
use chrono::prelude::*;
use i3ipc::event::inner::WindowChange;
use i3ipc::event::Event;
use i3ipc::I3EventListener;
use i3ipc::Subscription;
use std::error::Error;
use std::sync::mpsc::Sender;

#[derive(Clone)]
pub struct I3LogEvent {
    pub start_time: DateTime<Local>,
    pub window_id: u32,
    pub window_class: String,
    pub window_title: String,
}

impl I3LogEvent {
    fn new(window_id: u32, window_class: String, window_title: String) -> Self {
        I3LogEvent {
            start_time: Local::now(),
            window_id,
            window_class,
            window_title,
        }
    }
    pub fn from_tick(old_event: &Self) -> Self {
        I3LogEvent {
            start_time: Local::now(),
            window_id: old_event.window_id,
            window_class: old_event.window_class.clone(),
            window_title: old_event.window_title.clone(),
        }
    }
}

pub fn run(sender: Sender<super::time_tracker::LogEvent>) -> Result<(), Box<dyn Error>> {
    let mut i3_listener = I3EventListener::connect()?;

    let subs = [Subscription::Window];
    i3_listener.subscribe(&subs)?;
    let mut prev_new_window_id: Option<i32> = None;
    for event in i3_listener.listen() {
        if let Event::WindowEvent(e) = event? {
            let window_id = e.container.window.unwrap_or(-1);
            if window_id < 1 {
                continue;
            }
            // new window events get duplicated in the listen loop so we need
            // to track a "new" event and ensure we only actually emit the title
            // event
            match e.change {
                WindowChange::New => {
                    prev_new_window_id = Some(window_id);
                    continue;
                }
                WindowChange::Focus => {
                    if let Some(prev_window_id) = prev_new_window_id {
                        if prev_window_id == window_id {
                            prev_new_window_id = None;
                            continue;
                        }
                    }
                }
                _ => {}
            };
            prev_new_window_id = None;
            match e.change {
                WindowChange::Focus | WindowChange::Title => {
                    let mut window_class = "".into();
                    if let Some(properties) = e.container.window_properties {
                        if let Some(win_class) =
                            properties.get(&i3ipc::reply::WindowProperty::Class)
                        {
                            window_class = win_class.clone();
                        }
                    }
                    let window_title = e
                        .container
                        .name
                        .clone()
                        .unwrap_or_else(|| "Untitled".into());
                    let send_event = I3Event(I3LogEvent::new(
                        window_id as u32,
                        window_class,
                        window_title,
                    ));
                    sender.send(send_event)?;
                }
                _ => {}
            };
        }
    }
    Ok(())
}
