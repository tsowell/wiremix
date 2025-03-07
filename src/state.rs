//! Representation of PipeWire state.

use std::collections::HashMap;
use std::collections::HashSet;

use crate::command::Command;
use crate::event::MonitorEvent;
use crate::media_class::MediaClass;
use crate::object::ObjectId;

#[derive(Debug)]
pub struct Profile {
    pub index: i32,
    pub description: String,
    pub available: bool,
    pub classes: Vec<(MediaClass, Vec<i32>)>,
}

#[derive(Debug)]
pub struct EnumRoute {
    pub index: i32,
    pub description: String,
    pub available: bool,
    pub profiles: Vec<i32>,
    pub devices: Vec<i32>,
}

#[derive(Debug)]
pub struct Route {
    pub index: i32,
    pub device: i32,
    pub profiles: Vec<i32>,
    pub description: String,
    pub available: bool,
    pub volumes: Vec<f32>,
    pub mute: bool,
}

#[derive(Default, Debug)]
pub struct Device {
    pub id: ObjectId,
    pub object_serial: Option<i32>,
    pub name: Option<String>,
    pub nick: Option<String>,
    pub description: Option<String>,
    pub media_class: Option<MediaClass>,
    pub profile_index: Option<i32>,
    pub profiles: HashMap<i32, Profile>,
    pub routes: HashMap<i32, Route>,
    pub enum_routes: HashMap<i32, EnumRoute>,
}

#[derive(Default, Debug)]
pub struct Node {
    pub id: ObjectId,
    pub name: Option<String>,
    pub nick: Option<String>,
    pub description: Option<String>,
    pub media_class: Option<MediaClass>,
    pub media_name: Option<String>,
    pub object_serial: Option<i32>,
    pub volumes: Option<Vec<f32>>,
    pub mute: Option<bool>,
    pub peaks: Option<Vec<f32>>,
    pub rate: Option<u32>,
    pub positions: Option<Vec<u32>>,
    pub device_id: Option<ObjectId>,
    pub card_profile_device: Option<i32>,
}

impl Node {
    /// Update peaks with VU-meter-style ballistics
    pub fn update_peaks(&mut self, peaks: &Vec<f32>, samples: u32) {
        let Some(rate) = self.rate else {
            return;
        };

        // Initialize or resize current peaks.
        let peaks_ref = self.peaks.get_or_insert_default();
        if peaks_ref.len() != peaks.len() {
            // New length, clean slate.
            peaks_ref.clear();
        }
        // Make sure it's the right size.
        peaks_ref.resize(peaks.len(), 0.0);

        // Attack/release time of 300 ms
        let time_constant = 0.3;
        let coef =
            1.0 - (-(samples as f32) / (time_constant * rate as f32)).exp();

        // Update the peaks in-place.
        for (current_peak, new_peak) in peaks_ref.iter_mut().zip(peaks) {
            *current_peak += (new_peak - *current_peak) * coef
        }
    }
}

#[derive(Debug)]
pub struct Link {
    pub output: ObjectId,
    pub input: ObjectId,
}

#[derive(Default, Debug)]
pub struct Metadata {
    pub id: ObjectId,
    pub metadata_name: Option<String>,
    /// Properties for each subject
    pub properties: HashMap<u32, HashMap<String, String>>,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum StateDirty {
    #[default]
    Clean,
    PeaksOnly,
    Everything,
}

#[derive(Default, Debug)]
/// PipeWire state, maintained from
/// [`MonitorEvent`](`crate::event::MonitorEvent`)s from the
/// [`monitor`](`crate::monitor`) module.
///
/// This is primarily for maintaining a representation of the PipeWire state,
/// but [`Self::update()`] also returns [`Command`](`crate::command::Command`)s
/// for starting and stopping streaming because the
/// [`monitor`](`crate::monitor`) callbacks don't individually have enough
/// information to determine when that should happen.
pub struct State {
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links: HashMap<ObjectId, Link>,
    pub metadatas: HashMap<ObjectId, Metadata>,
    pub metadatas_by_name: HashMap<String, ObjectId>,
    /// Nodes waiting on object.serial before we can start capture
    pub pending_capture: HashSet<ObjectId>,
    /// Used to optimize view rebuilding based on what has changed
    pub dirty: StateDirty,
}

impl State {
    /// Update the state based on the supplied event.
    ///
    /// Returns a list of [`Command`](`crate::command::Command`)s to be
    /// executed based on the changes.
    pub fn update(&mut self, event: MonitorEvent) -> Vec<Command> {
        let mut commands = Vec::new();

        // Peaks updates are very frequent and easy to merge, so track if those
        // are the only updates done since the state was last Clean.
        match (self.dirty, &event) {
            (
                StateDirty::Clean | StateDirty::PeaksOnly,
                MonitorEvent::NodePeaks(..),
            ) => {
                self.dirty = StateDirty::PeaksOnly;
            }
            _ => {
                self.dirty = StateDirty::Everything;
            }
        }

        // Update
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
            MonitorEvent::DeviceObjectSerial(id, object_serial) => {
                self.device_entry(id).object_serial = Some(object_serial);
            }
            MonitorEvent::DeviceEnumProfile(
                id,
                index,
                description,
                available,
                classes,
            ) => {
                self.device_entry(id).profiles.insert(
                    index,
                    Profile {
                        index,
                        description,
                        available,
                        classes,
                    },
                );
            }
            MonitorEvent::DeviceProfile(id, index) => {
                self.device_entry(id).profile_index = Some(index);
            }
            MonitorEvent::DeviceRoute(
                id,
                index,
                device,
                profiles,
                description,
                available,
                volumes,
                mute,
            ) => {
                self.device_entry(id).routes.insert(
                    device,
                    Route {
                        index,
                        device,
                        profiles,
                        description,
                        available,
                        volumes,
                        mute,
                    },
                );
            }
            MonitorEvent::DeviceEnumRoute(
                id,
                index,
                description,
                available,
                profiles,
                devices,
            ) => {
                self.device_entry(id).enum_routes.insert(
                    index,
                    EnumRoute {
                        index,
                        description,
                        available,
                        profiles,
                        devices,
                    },
                );
            }
            MonitorEvent::NodeCardProfileDevice(id, card_profile_device) => {
                self.node_entry(id).card_profile_device =
                    Some(card_profile_device);
            }
            MonitorEvent::NodeDescription(id, description) => {
                self.node_entry(id).description = Some(description);
            }
            MonitorEvent::NodeDeviceId(id, device_id) => {
                self.node_entry(id).device_id = Some(device_id);
            }
            MonitorEvent::NodeMediaClass(id, media_class) => {
                let object_serial = {
                    let node = self.node_entry(id);
                    node.media_class = Some(media_class.clone());
                    node.object_serial
                };

                if self.is_node_auto_capturable(id) {
                    if object_serial.is_none() {
                        self.pending_capture.insert(id);
                    } else {
                        commands.extend(self.start_capture_command(&id));
                    }
                }
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
                if self.pending_capture.remove(&id)
                    && self.is_node_auto_capturable(id)
                {
                    commands.extend(self.start_capture_command(&id));
                }
            }
            MonitorEvent::NodePeaks(id, peaks, samples) => {
                self.node_entry(id).update_peaks(&peaks, samples);
            }
            MonitorEvent::NodeRate(id, rate) => {
                self.node_entry(id).rate = Some(rate);
            }
            MonitorEvent::NodePositions(id, positions) => {
                self.node_entry(id).positions = Some(positions);
            }
            MonitorEvent::NodeVolumes(id, volumes) => {
                self.node_entry(id).volumes = Some(volumes);
            }
            MonitorEvent::Link(id, output, input) => {
                if !self.inputs(input).contains(&output)
                    && self.is_node_capturable_on_link(input)
                {
                    commands.extend(self.start_capture_command(&input));
                }

                self.links.insert(id, Link { output, input });
            }
            MonitorEvent::MetadataMetadataName(id, metadata_name) => {
                self.metadata_entry(id).metadata_name =
                    Some(metadata_name.clone());
                self.metadatas_by_name.insert(metadata_name, id);
            }
            MonitorEvent::MetadataProperty(id, subject, key, value) => {
                let properties = self
                    .metadata_entry(id)
                    .properties
                    .entry(subject)
                    .or_default();
                match key {
                    Some(key) => {
                        match value {
                            Some(value) => {
                                properties.insert(key, value.clone())
                            }
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
                self.links.get(&id).inspect(|Link { input, .. }| {
                    if self.inputs(*input).len() == 1 {
                        // This is the last input link.
                        commands.extend(self.stop_capture_command(input));
                    }
                });

                self.devices.remove(&id);
                self.nodes.remove(&id);
                self.links.remove(&id);
                self.pending_capture.remove(&id);

                if let Some(metadata) = self.metadatas.remove(&id) {
                    if let Some(metadata_name) = metadata.metadata_name {
                        self.metadatas_by_name.remove(&metadata_name);
                    }
                }
            }
        }

        commands
    }

    pub fn get_metadata_by_name(
        &self,
        metadata_name: &str,
    ) -> Option<&Metadata> {
        self.metadatas
            .get(self.metadatas_by_name.get(metadata_name)?)
    }

    /// Should we capture this node once we see it?
    fn is_node_auto_capturable(&self, id: ObjectId) -> bool {
        self.nodes
            .get(&id)
            .and_then(|node| node.media_class.as_ref())
            .is_some_and(|media_class| {
                media_class.is_source()
                    || media_class.is_sink_input()
                    || media_class.is_source_output()
            })
    }

    /// Should we capture this node once it is linked to another node?
    fn is_node_capturable_on_link(&self, id: ObjectId) -> bool {
        self.nodes
            .get(&id)
            .and_then(|node| node.media_class.as_ref())
            .is_some_and(|media_class| {
                media_class.is_sink()
                    || media_class.is_source()
                    || media_class.is_sink_input()
                    || media_class.is_source_output()
            })
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

    /// Returns the objects that the given object outputs to.
    pub fn outputs(&self, id: ObjectId) -> Vec<ObjectId> {
        self.links
            .iter()
            .filter(|(_key, l)| l.output == id)
            .map(|(_key, l)| l.input)
            .collect()
    }

    /// Returns the objects that input to the given object.
    pub fn inputs(&self, id: ObjectId) -> Vec<ObjectId> {
        self.links
            .iter()
            .filter(|(_key, l)| l.input == id)
            .map(|(_key, l)| l.output)
            .collect()
    }

    pub fn start_capture_command(&self, input: &ObjectId) -> Option<Command> {
        let node = self.nodes.get(input)?;
        let object_serial = &node.object_serial?;
        let capture_sink =
            node.media_class.as_ref().is_some_and(|media_class| {
                media_class.is_sink() || media_class.is_source()
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
        let metadata_name = String::from("metadata0");
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
        let metadata_name = String::from("metadata0");
        state.update(MonitorEvent::MetadataMetadataName(
            obj_id,
            metadata_name.clone(),
        ));

        state.update(MonitorEvent::Removed(obj_id));

        assert!(state.metadatas.get(&obj_id).is_none());
        assert!(state.metadatas_by_name.get(&metadata_name).is_none());
        assert!(state.get_metadata_by_name(&metadata_name).is_none());
    }

    fn get_metadata_properties<'a>(
        state: &'a State,
        obj_id: &ObjectId,
        subject: u32,
    ) -> &'a HashMap<String, String> {
        state
            .metadatas
            .get(obj_id)
            .unwrap()
            .properties
            .get(&subject)
            .unwrap()
    }

    #[test]
    fn state_metadata_clear_property() {
        let mut state: State = Default::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(MonitorEvent::MetadataMetadataName(
            obj_id,
            metadata_name.clone(),
        ));

        let key = String::from("key");
        let value = String::from("value");

        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            0,
            Some(key.clone()),
            Some(value.clone()),
        ));
        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            1,
            Some(key.clone()),
            Some(value.clone()),
        ));
        assert_eq!(
            get_metadata_properties(&state, &obj_id, 0).get(&key),
            Some(&value)
        );
        assert_eq!(
            get_metadata_properties(&state, &obj_id, 1).get(&key),
            Some(&value)
        );

        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            0,
            Some(key.clone()),
            None,
        ));
        assert_eq!(get_metadata_properties(&state, &obj_id, 0).get(&key), None);
        assert_eq!(
            get_metadata_properties(&state, &obj_id, 1).get(&key),
            Some(&value)
        );
    }

    #[test]
    fn state_metadata_clear_all_properties() {
        let mut state: State = Default::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(MonitorEvent::MetadataMetadataName(
            obj_id,
            metadata_name.clone(),
        ));

        let key = String::from("key");
        let value = String::from("value");

        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            0,
            Some(key.clone()),
            Some(value.clone()),
        ));
        state.update(MonitorEvent::MetadataProperty(
            obj_id,
            1,
            Some(key.clone()),
            Some(value.clone()),
        ));
        assert!(!get_metadata_properties(&state, &obj_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &obj_id, 1).is_empty());

        state.update(MonitorEvent::MetadataProperty(obj_id, 0, None, None));

        assert!(get_metadata_properties(&state, &obj_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &obj_id, 1).is_empty());
    }
}
