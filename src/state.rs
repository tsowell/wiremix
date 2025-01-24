use std::collections::HashMap;
use std::collections::HashSet;

use crate::event::{MonitorEvent, ObjectId};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Profile {
    pub index: i32,
    pub description: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Route {
    pub index: i32,
    pub description: String,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Device {
    pub id: ObjectId,
    pub name: Option<String>,
    pub nick: Option<String>,
    pub description: Option<String>,
    pub media_class: Option<String>,
    pub profile_index: Option<i32>,
    pub profiles: HashMap<i32, Profile>,
    pub route_index: Option<i32>,
    pub routes: HashMap<i32, Route>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Node {
    pub id: ObjectId,
    pub name: Option<String>,
    pub nick: Option<String>,
    pub description: Option<String>,
    pub media_class: Option<String>,
    pub media_name: Option<String>,
    pub volumes: Option<Vec<f32>>,
    pub peaks: Option<Vec<f32>>,
    pub positions: Option<Vec<u32>>,
    pub device_id: Option<ObjectId>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Metadata {
    pub id: ObjectId,
    pub metadata_name: Option<String>,
    pub properties: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct State {
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links_output: HashMap<ObjectId, HashSet<ObjectId>>,
    pub links_input: HashMap<ObjectId, HashSet<ObjectId>>,
    pub metadatas: HashMap<ObjectId, Metadata>,
    pub metadatas_by_name: HashMap<String, ObjectId>,
}

impl State {
    pub fn update(&mut self, event: MonitorEvent) {
        match event {
            MonitorEvent::DeviceDescription(id, description) => {
                self.device_entry(id).description = Some(description);
            }
            MonitorEvent::DeviceMediaClass(id, media_class) => {
                self.device_entry(id).media_class = Some(media_class);
            }
            MonitorEvent::DeviceName(id, name) => {
                self.device_entry(id).name = Some(name);
            }
            MonitorEvent::DeviceNick(id, nick) => {
                self.device_entry(id).nick = Some(nick);
            }
            MonitorEvent::DeviceProfileDescription(id, index, description) => {
                self.device_entry(id)
                    .profiles
                    .insert(index, Profile { index, description });
            }
            MonitorEvent::DeviceProfileIndex(id, index) => {
                self.device_entry(id).profile_index = Some(index);
            }
            MonitorEvent::DeviceRouteDescription(id, index, description) => {
                self.device_entry(id)
                    .routes
                    .insert(index, Route { index, description });
            }
            MonitorEvent::DeviceRouteIndex(id, index) => {
                self.device_entry(id).route_index = Some(index);
            }
            MonitorEvent::NodeDescription(id, description) => {
                self.node_entry(id).description = Some(description);
            }
            MonitorEvent::NodeDeviceId(id, device_id) => {
                self.node_entry(id).device_id = Some(device_id);
            }
            MonitorEvent::NodeMediaClass(id, media_class) => {
                self.node_entry(id).media_class = Some(media_class);
            }
            MonitorEvent::NodeMediaName(id, media_name) => {
                self.node_entry(id).media_name = Some(media_name);
            }
            MonitorEvent::NodeName(id, name) => {
                self.node_entry(id).name = Some(name);
            }
            MonitorEvent::NodeNick(id, nick) => {
                self.node_entry(id).nick = Some(nick);
            }
            MonitorEvent::NodePeaks(id, peaks) => {
                self.node_entry(id).peaks = Some(peaks);
            }
            MonitorEvent::NodePositions(id, positions) => {
                self.node_entry(id).positions = Some(positions);
            }
            MonitorEvent::NodeVolumes(id, volumes) => {
                self.node_entry(id).volumes = Some(volumes);
            }
            MonitorEvent::Link(output, input) => {
                self.links_output.entry(output).or_default().insert(input);
                self.links_input.entry(input).or_default().insert(output);
            }
            MonitorEvent::MetadataMetadataName(id, metadata_name) => {
                self.metadata_entry(id).metadata_name =
                    Some(metadata_name.clone());
                self.metadatas_by_name.insert(metadata_name, id);
            }
            MonitorEvent::MetadataProperty(id, key, value) => {
                match value {
                    Some(value) => {
                        self.metadata_entry(id).properties.insert(key, value)
                    }
                    None => self.metadata_entry(id).properties.remove(&key),
                };
            }
            MonitorEvent::Removed(id) => {
                self.devices.remove(&id);
                self.nodes.remove(&id);
                self.links_output.remove(&id);
                self.links_input.remove(&id);
                if let Some(metadata) = self.metadatas.remove(&id) {
                    if let Some(metadata_name) = metadata.metadata_name {
                        self.metadatas_by_name.remove(&metadata_name);
                    }
                }
            }
        }
    }

    pub fn get_metadata_by_name(
        &self,
        metadata_name: &str,
    ) -> Option<&Metadata> {
        self.metadatas
            .get(self.metadatas_by_name.get(metadata_name)?)
    }

    fn node_entry(&mut self, id: ObjectId) -> &mut Node {
        self.nodes.entry(id).or_insert_with(|| Node {
            id,
            ..Default::default()
        })
    }

    fn device_entry(&mut self, id: ObjectId) -> &mut Device {
        self.devices.entry(id).or_insert_with(|| Device {
            id,
            ..Default::default()
        })
    }

    fn metadata_entry(&mut self, id: ObjectId) -> &mut Metadata {
        self.metadatas.entry(id).or_insert_with(|| Metadata {
            id,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_metadata_insert() {
        let mut state: State = Default::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = "metadata0".to_string();
        state.update(MonitorEvent::MetadataMetadataName(
            obj_id,
            metadata_name.clone(),
        ));

        let metadata = state.metadatas.get(&obj_id).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name.clone()));

        let metadata = state.get_metadata_by_name(&metadata_name).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name));
    }

    #[test]
    fn state_metadata_remove() {
        let mut state: State = Default::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = "metadata0".to_string();
        state.update(MonitorEvent::MetadataMetadataName(
            obj_id,
            metadata_name.clone(),
        ));

        state.update(MonitorEvent::Removed(obj_id));

        assert!(state.metadatas.get(&obj_id).is_none());
        assert!(state.metadatas_by_name.get(&metadata_name).is_none());
        assert!(state.get_metadata_by_name(&metadata_name).is_none());
    }

    #[test]
    fn state_metadata_clear_property() {
        let mut state: State = Default::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = "metadata0".to_string();
        state.update(MonitorEvent::MetadataMetadataName(
            obj_id,
            metadata_name.clone(),
        ));

        let key = "key".to_string();
        let value = "value".to_string();

        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            key.clone(),
            Some(value.clone()),
        ));
        assert_eq!(
            state.metadatas.get(&obj_id).unwrap().properties.get(&key),
            Some(&value)
        );

        state.update(MonitorEvent::MetadataProperty(obj_id, key.clone(), None));
        assert_eq!(
            state.metadatas.get(&obj_id).unwrap().properties.get(&key),
            None
        );
    }
}
