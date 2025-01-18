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

#[derive(Debug)]
pub enum InputMessage {
    Event(crossterm::event::Event),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum MonitorMessage {
    DeviceDescription(ObjectId, String),
    DeviceName(ObjectId, String),
    DeviceNick(ObjectId, String),
    DeviceProfileDescription(ObjectId, i32, String),
    DeviceProfileIndex(ObjectId, i32),
    DeviceRouteDescription(ObjectId, i32, String),
    DeviceRouteIndex(ObjectId, i32),

    NodeDescription(ObjectId, String),
    NodeMediaName(ObjectId, String),
    NodeName(ObjectId, String),
    NodeNick(ObjectId, String),
    NodePeak(ObjectId, f32),
    NodeVolume(ObjectId, f32),

    Link(ObjectId, ObjectId),

    Removed(ObjectId),
}

impl From<&LinkInfoRef> for MonitorMessage {
    fn from(link_info: &LinkInfoRef) -> Self {
        MonitorMessage::Link(
            ObjectId(link_info.output_node_id()),
            ObjectId(link_info.input_node_id()),
        )
    }
}

#[derive(Debug)]
pub enum Message {
    Input(InputMessage),
    Monitor(MonitorMessage),
    Error(String),
}

impl From<crossterm::event::Event> for Message {
    fn from(event: crossterm::event::Event) -> Self {
        Message::Input(InputMessage::Event(event))
    }
}
