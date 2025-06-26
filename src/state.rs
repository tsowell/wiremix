//! Representation of PipeWire state.

use std::collections::HashMap;

use crate::capture_manager::CaptureManager;
use crate::event::MonitorEvent;
use crate::monitor::PropertyStore;
use crate::object::ObjectId;

#[derive(Debug)]
pub struct Profile {
    pub index: i32,
    pub description: String,
    pub available: bool,
    pub classes: Vec<(String, Vec<i32>)>,
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
    pub props: PropertyStore,
    pub profile_index: Option<i32>,
    pub profiles: HashMap<i32, Profile>,
    pub routes: HashMap<i32, Route>,
    pub enum_routes: HashMap<i32, EnumRoute>,
}

#[derive(Default, Debug)]
pub struct Client {
    pub id: ObjectId,
    pub props: PropertyStore,
}

#[derive(Default, Debug)]
pub struct Node {
    pub id: ObjectId,
    pub props: PropertyStore,
    pub volumes: Option<Vec<f32>>,
    pub mute: Option<bool>,
    pub peaks: Option<Vec<f32>>,
    pub rate: Option<u32>,
    pub positions: Option<Vec<u32>>,
}

impl Node {
    /// Update peaks with VU-meter-style ballistics
    pub fn update_peaks(&mut self, peaks: &Vec<f32>, samples: u32) {
        let Some(rate) = self.rate else {
            return;
        };

        // Initialize or resize current peaks.
        let peaks_ref = self.peaks.get_or_insert_with(Default::default);
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
/// but [`Self::update()`] also invokes callbacks on a
/// [`CaptureManager`](`crate::capture_manager::CaptureManager`) for starting
/// and stopping streaming because the [`monitor`](`crate::monitor`) callbacks
/// don't individually have enough information to determine when that should
/// happen.
pub struct State {
    pub clients: HashMap<ObjectId, Client>,
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links: HashMap<ObjectId, Link>,
    pub metadatas: HashMap<ObjectId, Metadata>,
    pub metadatas_by_name: HashMap<String, ObjectId>,
    /// Used to optimize view rebuilding based on what has changed
    pub dirty: StateDirty,
}

impl State {
    /// Update the state based on the supplied event. Also invokes callbacks on
    /// a [`CaptureManager`](`crate::capture_manager::CaptureManager`) for
    /// managing stream capturing.
    pub fn update(
        &mut self,
        capture_manager: &mut CaptureManager,
        event: MonitorEvent,
    ) {
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
            MonitorEvent::ClientProperties(id, props) => {
                self.client_entry(id).props = props;
            }
            MonitorEvent::DeviceProperties(id, props) => {
                self.device_entry(id).props = props;
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
            MonitorEvent::NodeProperties(id, props) => {
                self.node_entry(id).props = props;

                if let Some(node) = self.nodes.get(&id) {
                    capture_manager.on_node(node);
                }
            }
            MonitorEvent::NodeMute(id, mute) => {
                self.node_entry(id).mute = Some(mute);
            }
            MonitorEvent::NodePeaks(id, peaks, samples) => {
                self.node_entry(id).update_peaks(&peaks, samples);
            }
            MonitorEvent::NodeRate(id, rate) => {
                self.node_entry(id).rate = Some(rate);
            }
            MonitorEvent::NodePositions(id, positions) => {
                if let Some(node) = self.nodes.get(&id) {
                    let changed = node
                        .positions
                        .as_ref()
                        .is_some_and(|p| *p != positions);
                    if changed {
                        capture_manager.on_positions_changed(node);
                    }
                }
                self.node_entry(id).positions = Some(positions);
            }
            MonitorEvent::NodeVolumes(id, volumes) => {
                self.node_entry(id).volumes = Some(volumes);
            }
            MonitorEvent::Link(id, output, input) => {
                if !self.inputs(input).contains(&output) {
                    if let Some(node) = self.nodes.get(&input) {
                        capture_manager.on_link(node);
                    }
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
                // Remove from links and stop capture if the last input link
                if let Some(Link { input, .. }) = self.links.remove(&id) {
                    if self.inputs(input).len() == 1 {
                        if let Some(node) = self.nodes.get(&input) {
                            capture_manager.on_removed(node);
                        }
                    }
                }

                self.devices.remove(&id);
                self.clients.remove(&id);
                if let Some(node) = self.nodes.remove(&id) {
                    capture_manager.on_removed(&node);
                }

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

    fn client_entry(&mut self, id: ObjectId) -> &mut Client {
        self.clients.entry(id).or_insert_with(|| Client {
            id,
            ..Default::default()
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_metadata_insert() {
        let mut state = State::default();
        let mut capture_manager = CaptureManager::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataMetadataName(obj_id, metadata_name.clone()),
        );

        let metadata = state.metadatas.get(&obj_id).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name.clone()));

        let metadata = state.get_metadata_by_name(&metadata_name).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name));
    }

    #[test]
    fn state_metadata_remove() {
        let mut state = State::default();
        let mut capture_manager = CaptureManager::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataMetadataName(obj_id, metadata_name.clone()),
        );

        state.update(&mut capture_manager, MonitorEvent::Removed(obj_id));

        assert!(!state.metadatas.contains_key(&obj_id));
        assert!(!state.metadatas_by_name.contains_key(&metadata_name));
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
        let mut state = State::default();
        let mut capture_manager = CaptureManager::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataMetadataName(obj_id, metadata_name.clone()),
        );

        let key = String::from("key");
        let value = String::from("value");

        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataProperty(
                obj_id,
                0,
                Some(key.clone()),
                Some(value.clone()),
            ),
        );
        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataProperty(
                obj_id,
                1,
                Some(key.clone()),
                Some(value.clone()),
            ),
        );
        assert_eq!(
            get_metadata_properties(&state, &obj_id, 0).get(&key),
            Some(&value)
        );
        assert_eq!(
            get_metadata_properties(&state, &obj_id, 1).get(&key),
            Some(&value)
        );

        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataProperty(obj_id, 0, Some(key.clone()), None),
        );
        assert_eq!(get_metadata_properties(&state, &obj_id, 0).get(&key), None);
        assert_eq!(
            get_metadata_properties(&state, &obj_id, 1).get(&key),
            Some(&value)
        );
    }

    #[test]
    fn state_metadata_clear_all_properties() {
        let mut state = State::default();
        let mut capture_manager = CaptureManager::default();
        let obj_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataMetadataName(obj_id, metadata_name.clone()),
        );

        let key = String::from("key");
        let value = String::from("value");

        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataProperty(
                obj_id,
                0,
                Some(key.clone()),
                Some(value.clone()),
            ),
        );
        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataProperty(
                obj_id,
                1,
                Some(key.clone()),
                Some(value.clone()),
            ),
        );
        assert!(!get_metadata_properties(&state, &obj_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &obj_id, 1).is_empty());

        state.update(
            &mut capture_manager,
            MonitorEvent::MetadataProperty(obj_id, 0, None, None),
        );

        assert!(get_metadata_properties(&state, &obj_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &obj_id, 1).is_empty());
    }
}
