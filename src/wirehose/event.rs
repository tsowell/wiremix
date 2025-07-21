use pipewire::link::LinkInfoRef;

use crate::wirehose::{ObjectId, PropertyStore};

#[derive(Debug)]
pub enum Event {
    State(StateEvent),
    Error(String),
    Ready,
}

#[derive(Debug)]
pub enum StateEvent {
    DeviceEnumRoute {
        object_id: ObjectId,
        index: i32,
        description: String,
        available: bool,
        profiles: Vec<i32>,
        devices: Vec<i32>,
    },
    DeviceEnumProfile {
        object_id: ObjectId,
        index: i32,
        description: String,
        available: bool,
        classes: Vec<(String, Vec<i32>)>,
    },
    DeviceProfile {
        object_id: ObjectId,
        index: i32,
    },
    DeviceProperties {
        object_id: ObjectId,
        props: PropertyStore,
    },
    DeviceRoute {
        object_id: ObjectId,
        index: i32,
        device: i32,
        profiles: Vec<i32>,
        description: String,
        available: bool,
        channel_volumes: Vec<f32>,
        mute: bool,
    },

    MetadataMetadataName {
        object_id: ObjectId,
        metadata_name: String,
    },
    MetadataProperty {
        object_id: ObjectId,
        subject: u32,
        key: Option<String>,
        value: Option<String>,
    },

    ClientProperties {
        object_id: ObjectId,
        props: PropertyStore,
    },

    NodePeaks {
        object_id: ObjectId,
        peaks: Vec<f32>,
        samples: u32,
    },
    NodePositions {
        object_id: ObjectId,
        positions: Vec<u32>,
    },
    NodeProperties {
        object_id: ObjectId,
        props: PropertyStore,
    },
    NodeRate {
        object_id: ObjectId,
        rate: u32,
    },
    NodeVolumes {
        object_id: ObjectId,
        volumes: Vec<f32>,
    },
    NodeMute {
        object_id: ObjectId,
        mute: bool,
    },

    Link {
        object_id: ObjectId,
        output_id: ObjectId,
        input_id: ObjectId,
    },

    StreamStopped {
        object_id: ObjectId,
    },

    Removed {
        object_id: ObjectId,
    },
}

impl From<&LinkInfoRef> for StateEvent {
    fn from(link_info: &LinkInfoRef) -> Self {
        StateEvent::Link {
            object_id: ObjectId::from_raw_id(link_info.id()),
            output_id: ObjectId::from_raw_id(link_info.output_node_id()),
            input_id: ObjectId::from_raw_id(link_info.input_node_id()),
        }
    }
}
