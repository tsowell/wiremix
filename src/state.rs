use std::collections::HashMap;

use crate::command::Command;
use crate::event::MonitorEvent;
use crate::object::ObjectId;

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
    pub route_device: Option<i32>,
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
    pub object_serial: Option<i32>,
    pub volumes: Option<Vec<f32>>,
    pub mute: Option<bool>,
    pub peaks: Option<Vec<f32>>,
    pub positions: Option<Vec<u32>>,
    pub device_id: Option<ObjectId>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Link {
    pub output: ObjectId,
    pub input: ObjectId,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct Metadata {
    pub id: ObjectId,
    pub metadata_name: Option<String>,
    // Properties for each subject
    pub properties: HashMap<u32, HashMap<String, String>>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct State {
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links: HashMap<ObjectId, Link>,
    pub metadatas: HashMap<ObjectId, Metadata>,
    pub metadatas_by_name: HashMap<String, ObjectId>,
}

impl State {
    pub fn update(&mut self, event: MonitorEvent) -> Option<Command> {
        let command = match event {
            MonitorEvent::Link(_, output, input)
                // Only restart if link is new.
                if !self.inputs(input).contains(&output) =>
            {
                self.restart_capture_command(&input)
            }
            MonitorEvent::Removed(id) => {
                self.links.get(&id).and_then(|Link { input, .. }| {
                    if self.inputs(*input).len() == 1 {
                        // This is the last input link.
                        self.stop_capture_command(input)
                    } else {
                        None
                    }
                })
            }
            _ => None,
        };

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
            MonitorEvent::DeviceProfile(id, index) => {
                self.device_entry(id).profile_index = Some(index);
            }
            MonitorEvent::DeviceRouteDescription(id, index, description) => {
                self.device_entry(id)
                    .routes
                    .insert(index, Route { index, description });
            }
            MonitorEvent::DeviceRoute(id, index, device) => {
                self.device_entry(id).route_index = Some(index);
                self.device_entry(id).route_device = Some(device);
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
            MonitorEvent::NodeMute(id, mute) => {
                self.node_entry(id).mute = Some(mute);
            }
            MonitorEvent::NodeName(id, name) => {
                self.node_entry(id).name = Some(name);
            }
            MonitorEvent::NodeNick(id, nick) => {
                self.node_entry(id).nick = Some(nick);
            }
            MonitorEvent::NodeObjectSerial(id, object_serial) => {
                self.node_entry(id).object_serial = Some(object_serial);
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
            MonitorEvent::Link(id, output, input) => {
                self.links.insert(id, Link { output, input });
            }
            MonitorEvent::MetadataMetadataName(id, metadata_name) => {
                self.metadata_entry(id).metadata_name =
                    Some(metadata_name.clone());
                self.metadatas_by_name.insert(metadata_name, id);
            }
            MonitorEvent::MetadataProperty(id, _subject, key, value) => {
                let properties = self
                    .metadata_entry(id)
                    .properties
                    .entry(_subject)
                    .or_default();
                match key {
                    Some(key) => {
                        match value {
                            Some(value) => properties.insert(key, value),
                            None => properties.remove(&key),
                        };
                    }
                    None => properties.clear(),
                };
            }
            MonitorEvent::StreamStopped(id) => {
                // It's likely that the node doesn't exist anymore.
                self.nodes.entry(id).and_modify(|node| node.peaks = None);
            }
            MonitorEvent::Removed(id) => {
                self.devices.remove(&id);
                self.nodes.remove(&id);
                self.links.remove(&id);

                if let Some(metadata) = self.metadatas.remove(&id) {
                    if let Some(metadata_name) = metadata.metadata_name {
                        self.metadatas_by_name.remove(&metadata_name);
                    }
                }
            }
        }

        command
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

    pub fn outputs(&self, id: ObjectId) -> Vec<ObjectId> {
        self.links
            .iter()
            .filter(|(_key, l)| l.output == id)
            .map(|(_key, l)| l.input)
            .collect()
    }

    pub fn inputs(&self, id: ObjectId) -> Vec<ObjectId> {
        self.links
            .iter()
            .filter(|(_key, l)| l.input == id)
            .map(|(_key, l)| l.output)
            .collect()
    }

    pub fn restart_capture_command(&self, input: &ObjectId) -> Option<Command> {
        let node = self.nodes.get(input)?;
        if node.media_class.as_ref().is_some_and(|c| {
            !matches!(
                c.as_str(),
                "Audio/Sink"
                    | "Audio/Source"
                    | "Stream/Output/Audio"
                    | "Stream/Input/Audio"
            )
        }) {
            return None;
        }
        let object_serial = &node.object_serial?;
        let capture_sink = node.media_class.as_ref().is_some_and(|c| {
            matches!(c.as_str(), "Audio/Sink" | "Audio/Source")
        });

        Some(Command::NodeCaptureStart(
            node.id,
            *object_serial,
            capture_sink,
        ))
    }

    pub fn stop_capture_command(&self, input: &ObjectId) -> Option<Command> {
        let node = self.nodes.get(input)?;

        Some(Command::NodeCaptureStop(node.id))
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
            0,
            Some(key.clone()),
            Some(value.clone()),
        ));
        assert_eq!(
            state.metadatas.get(&obj_id).unwrap().properties.get(&key),
            Some(&value)
        );

        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            0,
            Some(key.clone()),
            None,
        ));
        assert_eq!(
            state.metadatas.get(&obj_id).unwrap().properties.get(&key),
            None
        );
    }

    #[test]
    fn state_metadata_clear_all_properties() {
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
            0,
            Some(key.clone()),
            Some(value.clone()),
        ));
        assert!(!state.metadatas.get(&obj_id).unwrap().properties.is_empty());

        state.update(MonitorEvent::MetadataProperty(obj_id, 0, None, None));

        assert!(state.metadatas.get(&obj_id).unwrap().properties.is_empty());
    }
}
