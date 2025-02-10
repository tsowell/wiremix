use pipewire::link::LinkInfoRef;

use crate::media_class::MediaClass;
use crate::object::ObjectId;

#[allow(dead_code)]
#[derive(Debug)]
pub enum MonitorEvent {
    DeviceDescription(ObjectId, String),
    DeviceMediaClass(ObjectId, MediaClass),
    DeviceName(ObjectId, String),
    DeviceNick(ObjectId, String),
    DeviceProfileDescription(ObjectId, i32, String),
    DeviceProfile(ObjectId, i32),
    DeviceRoute(ObjectId, i32, i32, String, Vec<f32>, bool),

    MetadataMetadataName(ObjectId, String),
    MetadataProperty(ObjectId, u32, Option<String>, Option<String>),

    NodeCardProfileDevice(ObjectId, i32),
    NodeDescription(ObjectId, String),
    NodeDeviceId(ObjectId, ObjectId),
    NodeMediaClass(ObjectId, MediaClass),
    NodeMediaName(ObjectId, String),
    NodeName(ObjectId, String),
    NodeNick(ObjectId, String),
    NodeObjectSerial(ObjectId, i32),
    NodePeaks(ObjectId, Vec<f32>),
    NodePositions(ObjectId, Vec<u32>),
    NodeVolumes(ObjectId, Vec<f32>),
    NodeMute(ObjectId, bool),

    Link(ObjectId, ObjectId, ObjectId),

    StreamStopped(ObjectId),

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
