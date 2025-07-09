//! Input events for the application.
//!
//! These come from [`wirehose`](`crate::wirehose`) (PipeWire events) and from
//! [`input`](`crate::input`) (terminal input events).

use crate::wirehose::Event as PipewireEvent;

#[derive(Debug)]
pub enum Event {
    Input(crossterm::event::Event),
    Pipewire(PipewireEvent),
}

impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        Event::Input(event)
    }
}
