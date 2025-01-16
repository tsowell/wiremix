use libspa::utils::dict::DictRef;
use pipewire::{link::LinkInfoRef, registry::GlobalObject};

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u32);

impl From<&GlobalObject<&DictRef>> for ObjectId {
    fn from(obj: &GlobalObject<&DictRef>) -> Self {
        ObjectId(obj.id)
    }
}

pub enum InputMessage {
    Event(crossterm::event::Event),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum MonitorMessage {
    DeviceName(ObjectId, String),
    DeviceDescription(ObjectId, String),
    DeviceNick(ObjectId, String),
    DeviceRouteIndex(ObjectId, i32),
    DeviceRouteDescription(ObjectId, i32, String),
    DeviceProfileIndex(ObjectId, i32),
    DeviceProfileDescription(ObjectId, i32, String),

    NodeName(ObjectId, String),
    NodeDescription(ObjectId, String),
    NodeNick(ObjectId, String),
    NodeMediaName(ObjectId, String),
    NodeVolume(ObjectId, f32),
    NodePeak(ObjectId, f32),

    Link(ObjectId, ObjectId),

    Removed(ObjectId),

    Reset,
}

impl From<&LinkInfoRef> for MonitorMessage {
    fn from(link_info: &LinkInfoRef) -> Self {
        MonitorMessage::Link(
            ObjectId(link_info.output_node_id()),
            ObjectId(link_info.input_node_id()),
        )
    }
}

pub enum Message {
    Input(InputMessage),
    Monitor(MonitorMessage),
}

impl From<crossterm::event::Event> for Message {
    fn from(event: crossterm::event::Event) -> Self {
        Message::Input(InputMessage::Event(event))
    }
}
