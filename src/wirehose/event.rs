use std::sync::{atomic::AtomicU32, Arc};

use pipewire::link::LinkInfoRef;

use crate::wirehose::state::State;
use crate::wirehose::{ObjectId, PropertyStore};

/// Events emitted by the PipeWire monitoring thread.
#[derive(Debug)]
pub enum Event {
    /// The PipeWire state has changed
    State(StateEvent),
    /// An error occurred during monitoring
    Error(String),
    /// The [StateEvent]s representing the PipeWire state at the time of
    /// connection have been sent. wirehose is listening for changes now.
    Ready,
}

#[derive(Debug)]
/// PipeWire state change events.
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

    NodePositions {
        object_id: ObjectId,
        positions: Vec<u32>,
    },
    NodeProperties {
        object_id: ObjectId,
        props: PropertyStore,
    },
    NodeVolumes {
        object_id: ObjectId,
        volumes: Vec<f32>,
    },
    NodeMute {
        object_id: ObjectId,
        mute: bool,
    },
    NodeStreamStarted {
        object_id: ObjectId,
        rate: u32,
        peaks: Arc<[AtomicU32]>,
    },
    NodeStreamStopped {
        object_id: ObjectId,
    },
    NodePeaksDirty {
        object_id: ObjectId,
    },

    Link {
        object_id: ObjectId,
        output_id: ObjectId,
        input_id: ObjectId,
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

impl StateEvent {
    pub fn affected_objects(&self, state: &State) -> Vec<ObjectId> {
        match self {
            // A few special cases...
            StateEvent::Link {
                object_id,
                output_id,
                input_id,
            } => {
                // Include both ends of the link.
                vec![*object_id, *output_id, *input_id]
            }
            StateEvent::Removed { object_id } => {
                if let Some(link) = state.links.get(object_id) {
                    // If a link was removed, include both nodes.
                    vec![*object_id, link.output_id, link.input_id]
                } else {
                    vec![*object_id]
                }
            }
            // If a default source/sink is changing, return object ID 0 which
            // represents the core PipeWire state.
            StateEvent::MetadataProperty {
                object_id,
                subject: 0,
                ..
            } => state
                .metadatas
                .get(object_id)
                .filter(|metadata| {
                    metadata.metadata_name.as_deref() == Some("default")
                })
                .map(|_| vec![ObjectId::from_raw_id(0)])
                .unwrap_or_default(),
            StateEvent::MetadataProperty { .. } => vec![],

            // For the rest only the object_id is affected.
            StateEvent::DeviceEnumRoute { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::DeviceEnumProfile { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::DeviceProfile { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::DeviceProperties { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::DeviceRoute { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::MetadataMetadataName { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::ClientProperties { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::NodePositions { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::NodeProperties { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::NodeVolumes { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::NodeMute { object_id, .. } => vec![*object_id],
            StateEvent::NodeStreamStarted { object_id, .. } => {
                vec![*object_id]
            }
            StateEvent::NodeStreamStopped { object_id } => {
                vec![*object_id]
            }
            StateEvent::NodePeaksDirty { object_id } => {
                vec![*object_id]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affected_objects_link_includes_connected_nodes() {
        let state = State::default();
        let event = StateEvent::Link {
            object_id: ObjectId::from_raw_id(1),
            output_id: ObjectId::from_raw_id(2),
            input_id: ObjectId::from_raw_id(3),
        };
        assert_eq!(
            event.affected_objects(&state),
            vec![
                ObjectId::from_raw_id(1),
                ObjectId::from_raw_id(2),
                ObjectId::from_raw_id(3)
            ]
        );
    }

    #[test]
    fn affected_objects_removed_link_includes_connected_nodes() {
        let mut state = State::default();
        state.links.insert(
            ObjectId::from_raw_id(1),
            crate::wirehose::state::Link {
                output_id: ObjectId::from_raw_id(2),
                input_id: ObjectId::from_raw_id(3),
            },
        );

        let event = StateEvent::Removed {
            object_id: ObjectId::from_raw_id(1),
        };
        assert_eq!(
            event.affected_objects(&state),
            vec![
                ObjectId::from_raw_id(1),
                ObjectId::from_raw_id(2),
                ObjectId::from_raw_id(3)
            ]
        );
    }

    #[test]
    fn affected_objects_removed_non_link_returns_just_object_id() {
        let state = State::default();
        let event = StateEvent::Removed {
            object_id: ObjectId::from_raw_id(42),
        };
        assert_eq!(
            event.affected_objects(&state),
            vec![ObjectId::from_raw_id(42)]
        );
    }

    #[test]
    fn affected_objects_default_metadata_subject_0_returns_core_id() {
        let mut state = State::default();
        state.metadatas.insert(
            ObjectId::from_raw_id(10),
            crate::wirehose::state::Metadata {
                object_id: ObjectId::from_raw_id(10),
                metadata_name: Some("default".to_string()),
                properties: Default::default(),
            },
        );

        let event = StateEvent::MetadataProperty {
            object_id: ObjectId::from_raw_id(10),
            subject: 0,
            key: Some("default.audio.sink".to_string()),
            value: Some("42".to_string()),
        };
        assert_eq!(
            event.affected_objects(&state),
            vec![ObjectId::from_raw_id(0)]
        );
    }

    #[test]
    fn affected_objects_non_default_metadata_subject_0_returns_empty() {
        let mut state = State::default();
        state.metadatas.insert(
            ObjectId::from_raw_id(10),
            crate::wirehose::state::Metadata {
                object_id: ObjectId::from_raw_id(10),
                metadata_name: Some("route-settings".to_string()),
                properties: Default::default(),
            },
        );

        let event = StateEvent::MetadataProperty {
            object_id: ObjectId::from_raw_id(10),
            subject: 0,
            key: Some("some.key".to_string()),
            value: Some("some.value".to_string()),
        };
        assert!(event.affected_objects(&state).is_empty());
    }

    #[test]
    fn affected_objects_metadata_nonzero_subject_returns_empty() {
        let mut state = State::default();
        state.metadatas.insert(
            ObjectId::from_raw_id(10),
            crate::wirehose::state::Metadata {
                object_id: ObjectId::from_raw_id(10),
                metadata_name: Some("default".to_string()),
                properties: Default::default(),
            },
        );

        let event = StateEvent::MetadataProperty {
            object_id: ObjectId::from_raw_id(10),
            subject: 42,
            key: Some("target.node".to_string()),
            value: Some("100".to_string()),
        };
        assert!(event.affected_objects(&state).is_empty());
    }

    #[test]
    fn affected_objects_unknown_metadata_returns_empty() {
        let state = State::default();
        let event = StateEvent::MetadataProperty {
            object_id: ObjectId::from_raw_id(99),
            subject: 0,
            key: Some("key".to_string()),
            value: Some("value".to_string()),
        };
        assert!(event.affected_objects(&state).is_empty());
    }
}
