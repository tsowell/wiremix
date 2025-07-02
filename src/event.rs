//! Input events for the application.
//!
//! These come from [`monitor`](`crate::monitor`) (PipeWire events) and from
//! [`input`](`crate::input`) (terminal input events).

use crate::monitor::Event as MonitorEvent;

#[derive(Debug)]
pub enum Event {
    Input(crossterm::event::Event),
    Monitor(MonitorEvent),
}

impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        Event::Input(event)
    }
}
