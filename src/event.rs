use pipewire::link::LinkInfoRef;

use crate::object::ObjectId;

#[allow(dead_code)]
#[derive(Debug)]
pub enum MonitorEvent {
    DeviceDescription(ObjectId, String),
    DeviceMediaClass(ObjectId, String),
    DeviceName(ObjectId, String),
    DeviceNick(ObjectId, String),
    DeviceProfileDescription(ObjectId, i32, String),
    DeviceProfile(ObjectId, i32),
    DeviceRouteDescription(ObjectId, i32, String),
    DeviceRoute(ObjectId, i32, i32),

    MetadataMetadataName(ObjectId, String),
    MetadataProperty(ObjectId, String, Option<String>),

    NodeDescription(ObjectId, String),
    NodeDeviceId(ObjectId, ObjectId),
    NodeMediaClass(ObjectId, String),
    NodeMediaName(ObjectId, String),
    NodeName(ObjectId, String),
    NodeNick(ObjectId, String),
    NodeObjectSerial(ObjectId, i32),
    NodePeaks(ObjectId, Vec<f32>),
    NodePositions(ObjectId, Vec<u32>),
    NodeVolumes(ObjectId, Vec<f32>),

    Link(ObjectId, ObjectId, ObjectId),

    Removed(ObjectId),
}

impl From<&LinkInfoRef> for MonitorEvent {
    fn from(link_info: &LinkInfoRef) -> Self {
        MonitorEvent::Link(
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
    Error(String),
}

impl From<crossterm::event::Event> for Event {
    fn from(event: crossterm::event::Event) -> Self {
        Event::Input(event)
    }
}
