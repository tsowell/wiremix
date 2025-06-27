//! Input events for the application.
//!
//! These come from [`monitor`](`crate::monitor`) (PipeWire events) and from
//! [`input`](`crate::input`) (terminal input events).

use pipewire::link::LinkInfoRef;

use crate::monitor::{Event as MonitorEvent, ObjectId, StateEvent};

impl From<&LinkInfoRef> for StateEvent {
    fn from(link_info: &LinkInfoRef) -> Self {
        StateEvent::Link(
            ObjectId::from_raw_id(link_info.id()),
            ObjectId::from_raw_id(link_info.output_node_id()),
            ObjectId::from_raw_id(link_info.input_node_id()),
        )
    }
}

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
