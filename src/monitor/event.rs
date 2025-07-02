use pipewire::link::LinkInfoRef;

use crate::monitor::{ObjectId, PropertyStore};

#[derive(Debug)]
pub enum Event {
    State(StateEvent),
    Error(String),
    Ready,
}

#[derive(Debug)]
pub enum StateEvent {
    DeviceEnumRoute(ObjectId, i32, String, bool, Vec<i32>, Vec<i32>),
    DeviceEnumProfile(ObjectId, i32, String, bool, Vec<(String, Vec<i32>)>),
    DeviceProfile(ObjectId, i32),
    DeviceProperties(ObjectId, PropertyStore),
    DeviceRoute(ObjectId, i32, i32, Vec<i32>, String, bool, Vec<f32>, bool),

    MetadataMetadataName(ObjectId, String),
    MetadataProperty(ObjectId, u32, Option<String>, Option<String>),

    ClientProperties(ObjectId, PropertyStore),

    NodePeaks(ObjectId, Vec<f32>, u32),
    NodePositions(ObjectId, Vec<u32>),
    NodeProperties(ObjectId, PropertyStore),
    NodeRate(ObjectId, u32),
    NodeVolumes(ObjectId, Vec<f32>),
    NodeMute(ObjectId, bool),

    Link(ObjectId, ObjectId, ObjectId),

    StreamStopped(ObjectId),

    Removed(ObjectId),
}

impl From<&LinkInfoRef> for StateEvent {
    fn from(link_info: &LinkInfoRef) -> Self {
        StateEvent::Link(
            ObjectId::from_raw_id(link_info.id()),
            ObjectId::from_raw_id(link_info.output_node_id()),
            ObjectId::from_raw_id(link_info.input_node_id()),
        )
    }
}
