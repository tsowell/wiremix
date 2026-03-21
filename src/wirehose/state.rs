//! Representation of PipeWire state.

use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Arc};

use crate::atomic_f32::AtomicF32;
use crate::wirehose::{media_class, ObjectId, PropertyStore, StateEvent};

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
    pub info: HashMap<String, String>,
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
    pub object_id: ObjectId,
    pub props: PropertyStore,
    pub profile_index: Option<i32>,
    pub profiles: HashMap<i32, Profile>,
    pub routes: HashMap<i32, Route>,
    pub enum_routes: HashMap<i32, EnumRoute>,
}

#[derive(Default, Debug)]
pub struct Client {
    pub object_id: ObjectId,
    pub props: PropertyStore,
}

#[derive(Default, Debug)]
pub struct Node {
    pub object_id: ObjectId,
    pub props: PropertyStore,
    pub volumes: Option<Vec<f32>>,
    pub mute: Option<bool>,
    pub peaks: Option<Arc<[AtomicF32]>>,
    pub peaks_dirty: Arc<AtomicBool>,
    pub rate: Option<u32>,
    pub positions: Option<Vec<u32>>,
}

#[derive(Debug)]
pub struct Link {
    pub output_id: ObjectId,
    pub input_id: ObjectId,
}

#[derive(Default, Debug)]
pub struct Metadata {
    pub object_id: ObjectId,
    pub metadata_name: Option<String>,
    /// Properties for each subject
    pub properties: HashMap<u32, HashMap<String, String>>,
}

#[derive(Debug)]
pub enum CaptureEligibility {
    Eligible(ObjectId),
    Ineligible(ObjectId),
    NeedsRestart(ObjectId),
}

#[derive(Default)]
/// PipeWire state, maintained from [`StateEvent`]s from the
/// [`wirehose`](`crate::wirehose`) module.
///
/// This is primarily for maintaining a representation of the PipeWire state.
pub struct State {
    pub clients: HashMap<ObjectId, Client>,
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links: HashMap<ObjectId, Link>,
    pub metadatas: HashMap<ObjectId, Metadata>,
    pub metadatas_by_name: HashMap<String, ObjectId>,
}

impl State {
    /// Update the state based on the supplied event.
    ///
    /// Returns a [`CaptureEligibility`] if an object's capture eligibility
    /// might have changed.
    pub fn update(&mut self, event: StateEvent) -> Vec<CaptureEligibility> {
        let mut capture_eligibility = Vec::new();

        match event {
            StateEvent::ClientProperties { object_id, props } => {
                self.client_entry(object_id).props = props;
            }
            StateEvent::DeviceProperties { object_id, props } => {
                self.device_entry(object_id).props = props;
            }
            StateEvent::DeviceEnumProfile {
                object_id,
                index,
                description,
                available,
                classes,
            } => {
                self.device_entry(object_id).profiles.insert(
                    index,
                    Profile {
                        index,
                        description,
                        available,
                        classes,
                    },
                );
            }
            StateEvent::DeviceProfile { object_id, index } => {
                self.device_entry(object_id).profile_index = Some(index);
            }
            StateEvent::DeviceRoute {
                object_id: id,
                index,
                device,
                profiles,
                description,
                available,
                channel_volumes,
                mute,
            } => {
                self.device_entry(id).routes.insert(
                    device,
                    Route {
                        index,
                        device,
                        profiles,
                        description,
                        available,
                        volumes: channel_volumes,
                        mute,
                    },
                );
            }
            StateEvent::DeviceEnumRoute {
                object_id,
                index,
                description,
                available,
                profiles,
                devices,
                info,
            } => {
                self.device_entry(object_id).enum_routes.insert(
                    index,
                    EnumRoute {
                        index,
                        description,
                        available,
                        profiles,
                        devices,
                        info,
                    },
                );
            }
            StateEvent::NodeProperties { object_id, props } => {
                self.node_entry(object_id).props = props;

                if let Some(node) = self.nodes.get(&object_id) {
                    capture_eligibility.extend(self.on_node(node));
                }
            }
            StateEvent::NodeMute { object_id, mute } => {
                self.node_entry(object_id).mute = Some(mute);
            }
            StateEvent::NodePositions {
                object_id,
                positions,
            } => {
                if let Some(node) = self.nodes.get(&object_id) {
                    let changed = node
                        .positions
                        .as_ref()
                        .is_some_and(|p| *p != positions);
                    if changed {
                        capture_eligibility.push(
                            CaptureEligibility::NeedsRestart(node.object_id),
                        );
                    }
                }
                self.node_entry(object_id).positions = Some(positions);
            }
            StateEvent::NodeVolumes { object_id, volumes } => {
                self.node_entry(object_id).volumes = Some(volumes);
            }
            StateEvent::NodeStreamStarted {
                object_id,
                rate,
                peaks,
            } => {
                self.node_entry(object_id).rate = Some(rate);
                self.node_entry(object_id).peaks = Some(peaks);
            }
            StateEvent::NodeStreamStopped { object_id } => {
                // It's likely that the node doesn't exist anymore.
                self.nodes
                    .entry(object_id)
                    .and_modify(|node| node.peaks = None);
            }
            StateEvent::NodePeaksDirty { object_id: _ } => {
                // This message just wakes up the App.
            }
            StateEvent::Link {
                object_id,
                output_id,
                input_id,
            } => {
                if !self.inputs(input_id).contains(&output_id) {
                    if let Some(node) = self.nodes.get(&input_id) {
                        capture_eligibility.extend(self.on_link(node));
                    }
                }

                self.links.insert(
                    object_id,
                    Link {
                        output_id,
                        input_id,
                    },
                );
            }
            StateEvent::MetadataMetadataName {
                object_id,
                metadata_name,
            } => {
                self.metadata_entry(object_id).metadata_name =
                    Some(metadata_name.clone());
                self.metadatas_by_name.insert(metadata_name, object_id);
            }
            StateEvent::MetadataProperty {
                object_id,
                subject,
                key,
                value,
            } => {
                let properties = self
                    .metadata_entry(object_id)
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
            StateEvent::Removed { object_id } => {
                // Remove from links and stop capture if the last input link
                if let Some(Link { input_id, .. }) =
                    self.links.remove(&object_id)
                {
                    if self.inputs(input_id).is_empty() {
                        if let Some(node) = self.nodes.get(&input_id) {
                            capture_eligibility.push(
                                CaptureEligibility::Ineligible(node.object_id),
                            );
                        }
                    }
                }

                self.devices.remove(&object_id);
                self.clients.remove(&object_id);
                if let Some(node) = self.nodes.remove(&object_id) {
                    capture_eligibility
                        .push(CaptureEligibility::Ineligible(node.object_id));
                }

                if let Some(metadata) = self.metadatas.remove(&object_id) {
                    if let Some(metadata_name) = metadata.metadata_name {
                        self.metadatas_by_name.remove(&metadata_name);
                    }
                }
            }
        }

        capture_eligibility
    }

    pub fn get_metadata_by_name(
        &self,
        metadata_name: &str,
    ) -> Option<&Metadata> {
        self.metadatas
            .get(self.metadatas_by_name.get(metadata_name)?)
    }

    fn client_entry(&mut self, object_id: ObjectId) -> &mut Client {
        self.clients.entry(object_id).or_insert_with(|| Client {
            object_id,
            ..Default::default()
        })
    }

    fn node_entry(&mut self, object_id: ObjectId) -> &mut Node {
        self.nodes.entry(object_id).or_insert_with(|| Node {
            object_id,
            ..Default::default()
        })
    }

    fn device_entry(&mut self, object_id: ObjectId) -> &mut Device {
        self.devices.entry(object_id).or_insert_with(|| Device {
            object_id,
            ..Default::default()
        })
    }

    fn metadata_entry(&mut self, object_id: ObjectId) -> &mut Metadata {
        self.metadatas.entry(object_id).or_insert_with(|| Metadata {
            object_id,
            ..Default::default()
        })
    }

    /// Returns the objects that the given object outputs to.
    pub fn outputs(&self, object_id: ObjectId) -> Vec<ObjectId> {
        self.links
            .iter()
            .filter(|(_key, l)| l.output_id == object_id)
            .map(|(_key, l)| l.input_id)
            .collect()
    }

    /// Returns the objects that input to the given object.
    pub fn inputs(&self, object_id: ObjectId) -> Vec<ObjectId> {
        self.links
            .iter()
            .filter(|(_key, l)| l.input_id == object_id)
            .map(|(_key, l)| l.output_id)
            .collect()
    }

    pub fn active_route<'a>(
        &'a self,
        device: &'a Device,
        card_device: i32,
    ) -> Option<&'a Route> {
        let profile_index = device.profile_index?;

        device
            .routes
            .get(&card_device)
            .filter(|route| route.profiles.contains(&profile_index))
    }

    pub fn active_route_for_node<'a>(
        &'a self,
        node: &'a Node,
    ) -> Option<(&'a Device, &'a Route, i32)> {
        let device = self.devices.get(node.props.device_id()?)?;
        let card_device = *node.props.card_profile_device()?;
        let route = self.active_route(device, card_device)?;
        Some((device, route, card_device))
    }

    pub fn route_targets<'a>(
        &'a self,
        device: &'a Device,
        media_class: &str,
    ) -> Option<Vec<(&'a EnumRoute, i32)>> {
        let profile_index = device.profile_index?;
        let profile = device.profiles.get(&profile_index)?;
        let profile_devices = profile
            .classes
            .iter()
            .find_map(|(mc, devices)| (mc == media_class).then_some(devices))?;

        Some(
            device
                .enum_routes
                .values()
                .filter_map(|route| {
                    if !route.profiles.contains(&profile_index) {
                        return None;
                    }
                    let route_device = route
                        .devices
                        .iter()
                        .find(|route_device| profile_devices.contains(route_device))?;
                    Some((route, *route_device))
                })
                .collect(),
        )
    }

    pub fn endpoint_physical_label<'a>(
        &'a self,
        node: &'a Node,
    ) -> Option<&'a str> {
        let media_class = node.props.media_class()?;
        if !(
            media_class::is_sink(media_class)
                || media_class::is_source(media_class)
        ) {
            return None;
        }

        if let Some(node_nick) =
            node.props.node_nick().filter(|node_nick| !node_nick.is_empty())
        {
            return Some(node_nick.as_str());
        }

        let (device, route, _) = self.active_route_for_node(node)?;
        device.enum_routes.get(&route.index)?.physical_label()
    }

    pub fn device_profile_physical_label<'a>(
        &'a self,
        device: &'a Device,
        profile_index: i32,
    ) -> Option<&'a str> {
        let profile = device.profiles.get(&profile_index)?;
        let profile_devices: Vec<i32> = profile
            .classes
            .iter()
            .flat_map(|(_, devices)| devices.iter().copied())
            .collect();

        let mut matching_routes = device.enum_routes.values().filter(|route| {
            route.profiles.contains(&profile_index)
                && route
                    .devices
                    .iter()
                    .any(|route_device| profile_devices.contains(route_device))
        });

        let route = matching_routes.next()?;
        if matching_routes.next().is_some() {
            return None;
        }

        route.physical_label()
    }

    pub fn resolve_computed_node_key<'a>(
        &'a self,
        node: &'a Node,
        key: &str,
    ) -> Option<&'a str> {
        match key {
            "physical.endpoint.label" => self.endpoint_physical_label(node),
            _ => None,
        }
    }

    /// Call when a node's capture eligibility might have changed.
    fn on_node(&self, node: &Node) -> Option<CaptureEligibility> {
        if !node
            .props
            .media_class()
            .as_ref()
            .is_some_and(|media_class| {
                media_class::is_source(media_class)
                    || media_class::is_sink_input(media_class)
                    || media_class::is_source_output(media_class)
            })
        {
            return None;
        }

        node.props.object_serial()?;

        Some(CaptureEligibility::Eligible(node.object_id))
    }

    /// Call when a node gets a new input link.
    fn on_link(&self, node: &Node) -> Option<CaptureEligibility> {
        if !node
            .props
            .media_class()
            .as_ref()
            .is_some_and(|media_class| {
                media_class::is_sink(media_class)
                    || media_class::is_source(media_class)
                    || media_class::is_sink_input(media_class)
                    || media_class::is_source_output(media_class)
            })
        {
            return None;
        }

        Some(CaptureEligibility::NeedsRestart(node.object_id))
    }
}

impl EnumRoute {
    pub fn info(&self, key: &str) -> Option<&str> {
        self.info.get(key).map(String::as_str)
    }

    pub fn physical_label(&self) -> Option<&str> {
        self.info("device.product.name")
            .filter(|label| !label.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_device_with_profile(state: &mut State, object_id: ObjectId) {
        state.update(StateEvent::DeviceProperties {
            object_id,
            props: PropertyStore::default(),
        });
        state.update(StateEvent::DeviceEnumProfile {
            object_id,
            index: 1,
            description: String::from("Digital Stereo Output"),
            available: true,
            classes: vec![(String::from("Audio/Sink"), vec![0])],
        });
        state.update(StateEvent::DeviceProfile {
            object_id,
            index: 1,
        });
        state.update(StateEvent::DeviceRoute {
            object_id,
            index: 7,
            device: 0,
            profiles: vec![1],
            description: String::from("HDMI"),
            available: true,
            channel_volumes: vec![1.0, 1.0],
            mute: false,
        });
    }

    fn create_node(
        state: &mut State,
        object_id: ObjectId,
        media_class: &str,
        object_serial: u64,
    ) {
        let mut props = PropertyStore::default();
        props.set_media_class(String::from(media_class));
        props.set_object_serial(object_serial);
        state.update(StateEvent::NodeProperties { object_id, props });
    }

    #[test]
    fn state_metadata_insert() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(StateEvent::MetadataMetadataName {
            object_id,
            metadata_name: metadata_name.clone(),
        });

        let metadata = state.metadatas.get(&object_id).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name.clone()));

        let metadata = state.get_metadata_by_name(&metadata_name).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name));
    }

    #[test]
    fn state_metadata_remove() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(StateEvent::MetadataMetadataName {
            object_id,
            metadata_name: metadata_name.clone(),
        });

        state.update(StateEvent::Removed { object_id });

        assert!(!state.metadatas.contains_key(&object_id));
        assert!(!state.metadatas_by_name.contains_key(&metadata_name));
        assert!(state.get_metadata_by_name(&metadata_name).is_none());
    }

    fn get_metadata_properties<'a>(
        state: &'a State,
        object_id: &ObjectId,
        subject: u32,
    ) -> &'a HashMap<String, String> {
        state
            .metadatas
            .get(object_id)
            .unwrap()
            .properties
            .get(&subject)
            .unwrap()
    }

    #[test]
    fn state_metadata_clear_property() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(StateEvent::MetadataMetadataName {
            object_id,
            metadata_name: metadata_name.clone(),
        });

        let key = String::from("key");
        let value = String::from("value");

        state.update(StateEvent::MetadataProperty {
            object_id,
            subject: 0,
            key: Some(key.clone()),
            value: Some(value.clone()),
        });
        state.update(StateEvent::MetadataProperty {
            object_id,
            subject: 1,
            key: Some(key.clone()),
            value: Some(value.clone()),
        });
        assert_eq!(
            get_metadata_properties(&state, &object_id, 0).get(&key),
            Some(&value)
        );
        assert_eq!(
            get_metadata_properties(&state, &object_id, 1).get(&key),
            Some(&value)
        );

        state.update(StateEvent::MetadataProperty {
            object_id,
            subject: 0,
            key: Some(key.clone()),
            value: None,
        });
        assert_eq!(
            get_metadata_properties(&state, &object_id, 0).get(&key),
            None
        );
        assert_eq!(
            get_metadata_properties(&state, &object_id, 1).get(&key),
            Some(&value)
        );
    }

    #[test]
    fn state_metadata_clear_all_properties() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(StateEvent::MetadataMetadataName {
            object_id,
            metadata_name: metadata_name.clone(),
        });

        let key = String::from("key");
        let value = String::from("value");

        state.update(StateEvent::MetadataProperty {
            object_id,
            subject: 0,
            key: Some(key.clone()),
            value: Some(value.clone()),
        });
        state.update(StateEvent::MetadataProperty {
            object_id,
            subject: 1,
            key: Some(key.clone()),
            value: Some(value.clone()),
        });
        assert!(!get_metadata_properties(&state, &object_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &object_id, 1).is_empty());

        state.update(StateEvent::MetadataProperty {
            object_id,
            subject: 0,
            key: None,
            value: None,
        });

        assert!(get_metadata_properties(&state, &object_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &object_id, 1).is_empty());
    }

    #[test]
    fn endpoint_physical_label_prefers_node_nick() {
        let mut state = State::default();
        let device_id = ObjectId::from_raw_id(10);
        let node_id = ObjectId::from_raw_id(11);

        create_device_with_profile(&mut state, device_id);
        state.update(StateEvent::DeviceEnumRoute {
            object_id: device_id,
            index: 7,
            description: String::from("HDMI"),
            available: true,
            profiles: vec![1],
            devices: vec![0],
            info: HashMap::from([(
                String::from("device.product.name"),
                String::from("Route label"),
            )]),
        });

        let mut props = PropertyStore::default();
        props.set_media_class(String::from("Audio/Sink"));
        props.set_node_name(String::from("sink"));
        props.set_node_nick(String::from("Node label"));
        props.set_device_id(device_id);
        props.set_card_profile_device(0);
        props.set_object_serial(11);
        state.update(StateEvent::NodeProperties {
            object_id: node_id,
            props,
        });

        let node = state.nodes.get(&node_id).unwrap();
        assert_eq!(state.endpoint_physical_label(node), Some("Node label"));
    }

    #[test]
    fn device_profile_physical_label_requires_unique_route() {
        let mut state = State::default();
        let device_id = ObjectId::from_raw_id(10);

        create_device_with_profile(&mut state, device_id);
        state.update(StateEvent::DeviceEnumRoute {
            object_id: device_id,
            index: 7,
            description: String::from("HDMI"),
            available: true,
            profiles: vec![1],
            devices: vec![0],
            info: HashMap::from([(
                String::from("device.product.name"),
                String::from("DELL U2723QE"),
            )]),
        });

        let device = state.devices.get(&device_id).unwrap();
        assert_eq!(
            state.device_profile_physical_label(device, 1),
            Some("DELL U2723QE")
        );
    }

    #[test]
    fn device_profile_physical_label_is_none_for_multiple_routes() {
        let mut state = State::default();
        let device_id = ObjectId::from_raw_id(10);

        create_device_with_profile(&mut state, device_id);
        state.update(StateEvent::DeviceEnumRoute {
            object_id: device_id,
            index: 7,
            description: String::from("HDMI 1"),
            available: true,
            profiles: vec![1],
            devices: vec![0],
            info: HashMap::from([(
                String::from("device.product.name"),
                String::from("DELL U2723QE"),
            )]),
        });
        state.update(StateEvent::DeviceEnumRoute {
            object_id: device_id,
            index: 8,
            description: String::from("HDMI 2"),
            available: true,
            profiles: vec![1],
            devices: vec![0],
            info: HashMap::from([(
                String::from("device.product.name"),
                String::from("PA32QCV"),
            )]),
        });

        let device = state.devices.get(&device_id).unwrap();
        assert_eq!(state.device_profile_physical_label(device, 1), None);
    }

    #[test]
    fn capture_eligible_for_stream() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(1);

        let result = state.update(StateEvent::NodeProperties {
            object_id,
            props: {
                let mut props = PropertyStore::default();
                props.set_media_class(String::from("Stream/Output/Audio"));
                props.set_object_serial(100);
                props
            },
        });

        assert!(matches!(
            result.as_slice(),
            [CaptureEligibility::Eligible(id)] if *id == object_id
        ));
    }

    #[test]
    fn capture_eligible_requires_object_serial() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(1);

        let result = state.update(StateEvent::NodeProperties {
            object_id,
            props: {
                let mut props = PropertyStore::default();
                props.set_media_class(String::from("Stream/Output/Audio"));
                // No object_serial set
                props
            },
        });

        assert!(result.is_empty());
    }

    #[test]
    fn capture_needs_restart_on_positions_change() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(1);

        create_node(&mut state, object_id, "Stream/Output/Audio", 100);
        state.update(StateEvent::NodePositions {
            object_id,
            positions: vec![1, 2],
        });

        // Change positions
        let result = state.update(StateEvent::NodePositions {
            object_id,
            positions: vec![1, 2, 3],
        });

        assert!(matches!(
            result.as_slice(),
            [CaptureEligibility::NeedsRestart(id)] if *id == object_id
        ));
    }

    #[test]
    fn capture_no_restart_on_same_positions() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(1);

        create_node(&mut state, object_id, "Stream/Output/Audio", 100);
        state.update(StateEvent::NodePositions {
            object_id,
            positions: vec![1, 2],
        });

        // Same positions
        let result = state.update(StateEvent::NodePositions {
            object_id,
            positions: vec![1, 2],
        });

        assert!(result.is_empty());
    }

    #[test]
    fn capture_needs_restart_on_link_to_sink() {
        let mut state = State::default();
        let stream_id = ObjectId::from_raw_id(1);
        let sink_id = ObjectId::from_raw_id(2);

        create_node(&mut state, stream_id, "Stream/Output/Audio", 100);
        create_node(&mut state, sink_id, "Audio/Sink", 101);

        let result = state.update(StateEvent::Link {
            object_id: ObjectId::from_raw_id(10),
            output_id: stream_id,
            input_id: sink_id,
        });

        assert!(matches!(
            result.as_slice(),
            [CaptureEligibility::NeedsRestart(id)] if *id == sink_id
        ));
    }

    #[test]
    fn capture_ineligible_on_node_removed() {
        let mut state = State::default();
        let object_id = ObjectId::from_raw_id(1);

        create_node(&mut state, object_id, "Stream/Output/Audio", 100);

        let result = state.update(StateEvent::Removed { object_id });

        assert!(matches!(
            result.as_slice(),
            [CaptureEligibility::Ineligible(id)] if *id == object_id
        ));
    }

    #[test]
    fn capture_ineligible_on_last_link_removed() {
        let mut state = State::default();
        let stream_id = ObjectId::from_raw_id(1);
        let sink_id = ObjectId::from_raw_id(2);
        let link_id = ObjectId::from_raw_id(10);

        create_node(&mut state, stream_id, "Stream/Output/Audio", 100);
        create_node(&mut state, sink_id, "Audio/Sink", 101);
        state.update(StateEvent::Link {
            object_id: link_id,
            output_id: stream_id,
            input_id: sink_id,
        });

        let result = state.update(StateEvent::Removed { object_id: link_id });

        assert!(matches!(
            result.as_slice(),
            [CaptureEligibility::Ineligible(id)] if *id == sink_id
        ));
    }
}
