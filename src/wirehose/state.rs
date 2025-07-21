//! Representation of PipeWire state.

use std::collections::{HashMap, HashSet};

use crate::wirehose::{
    command::Command, media_class, CommandSender, ObjectId, PropertyStore,
    StateEvent,
};

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
    pub peaks: Option<Vec<f32>>,
    pub rate: Option<u32>,
    pub positions: Option<Vec<u32>>,
}

/// Trait for processing peaks in order to implement effects like ballistics.
pub trait PeakProcessor {
    fn process_peak(
        &self,
        current_peak: f32,
        previous_peak: f32,
        sample_count: u32,
        sample_rate: u32,
    ) -> f32;
}

impl<F> PeakProcessor for F
where
    F: Fn(f32, f32, u32, u32) -> f32,
{
    fn process_peak(
        &self,
        current_peak: f32,
        previous_peak: f32,
        sample_count: u32,
        sample_rate: u32,
    ) -> f32 {
        self(current_peak, previous_peak, sample_count, sample_rate)
    }
}

impl Node {
    /// Update peaks with an optional peak processor for ballistics or other
    /// effects.
    pub fn update_peaks(
        &mut self,
        peaks: &Vec<f32>,
        samples: u32,
        peak_processor: Option<&dyn PeakProcessor>,
    ) {
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

        for (current_peak, new_peak) in peaks_ref.iter_mut().zip(peaks) {
            match peak_processor {
                Some(peak_processor) => {
                    *current_peak = peak_processor.process_peak(
                        *current_peak,
                        *new_peak,
                        rate,
                        samples,
                    );
                }
                None => {
                    *current_peak = *new_peak;
                }
            }
        }
    }
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

#[derive(Default)]
/// PipeWire state, maintained from [`StateEvent`]s from the
/// [`wirehose`](`crate::wirehose`) module.
///
/// This is primarily for maintaining a representation of the PipeWire state,
/// but [`Self::update()`] also handles capture management for starting
/// and stopping streaming because the [`wirehose`](`crate::wirehose`)
/// callbacks don't individually have enough information to determine when that
/// should happen.
pub struct State {
    pub clients: HashMap<ObjectId, Client>,
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links: HashMap<ObjectId, Link>,
    pub metadatas: HashMap<ObjectId, Metadata>,
    pub metadatas_by_name: HashMap<String, ObjectId>,
    peak_processor: Option<Box<dyn PeakProcessor>>,
    capturing: Option<HashSet<ObjectId>>,
}

impl State {
    /// Provide a peak processor for setting peak levels.
    pub fn with_peak_processor(
        mut self,
        peak_processor: Box<dyn PeakProcessor>,
    ) -> Self {
        self.peak_processor = Some(peak_processor);
        self
    }

    /// Enable stream capturing.
    pub fn with_capture(mut self, enable: bool) -> Self {
        self.capturing = enable.then_some(Default::default());
        self
    }

    /// Update the state based on the supplied event. Also handles capture
    /// management for starting and stopping streaming.
    pub fn update(&mut self, wirehose: &dyn CommandSender, event: StateEvent) {
        let mut commands = Vec::<Command>::new();

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
            } => {
                self.device_entry(object_id).enum_routes.insert(
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
            StateEvent::NodeProperties { object_id, props } => {
                self.node_entry(object_id).props = props;

                if let Some(node) = self.nodes.get(&object_id) {
                    commands.extend(self.on_node(node));
                }
            }
            StateEvent::NodeMute { object_id, mute } => {
                self.node_entry(object_id).mute = Some(mute);
            }
            StateEvent::NodePeaks {
                object_id,
                peaks,
                samples,
            } => {
                let node =
                    self.nodes.entry(object_id).or_insert_with(|| Node {
                        object_id,
                        ..Default::default()
                    });
                let peak_processor = self.peak_processor.as_deref();
                node.update_peaks(&peaks, samples, peak_processor);
            }
            StateEvent::NodeRate { object_id, rate } => {
                self.node_entry(object_id).rate = Some(rate);
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
                        commands.extend(self.on_positions_changed(node));
                    }
                }
                self.node_entry(object_id).positions = Some(positions);
            }
            StateEvent::NodeVolumes { object_id, volumes } => {
                self.node_entry(object_id).volumes = Some(volumes);
            }
            StateEvent::Link {
                object_id,
                output_id,
                input_id,
            } => {
                if !self.inputs(input_id).contains(&output_id) {
                    if let Some(node) = self.nodes.get(&input_id) {
                        commands.extend(self.on_link(node));
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
            StateEvent::StreamStopped { object_id } => {
                // It's likely that the node doesn't exist anymore.
                self.nodes
                    .entry(object_id)
                    .and_modify(|node| node.peaks = None);
            }
            StateEvent::Removed { object_id } => {
                // Remove from links and stop capture if the last input link
                if let Some(Link { input_id, .. }) =
                    self.links.remove(&object_id)
                {
                    if self.inputs(input_id).len() == 1 {
                        if let Some(node) = self.nodes.get(&input_id) {
                            commands.extend(self.on_removed(node));
                        }
                    }
                }

                self.devices.remove(&object_id);
                self.clients.remove(&object_id);
                if let Some(node) = self.nodes.remove(&object_id) {
                    commands.extend(self.on_removed(&node));
                }

                if let Some(metadata) = self.metadatas.remove(&object_id) {
                    if let Some(metadata_name) = metadata.metadata_name {
                        self.metadatas_by_name.remove(&metadata_name);
                    }
                }
            }
        }

        if let Some(capturing) = &mut self.capturing {
            for command in commands.into_iter() {
                match command {
                    Command::NodeCaptureStart(
                        obj_id,
                        object_serial,
                        capture_sink,
                    ) => {
                        capturing.insert(obj_id);
                        wirehose.node_capture_start(
                            obj_id,
                            object_serial,
                            capture_sink,
                        );
                    }
                    Command::NodeCaptureStop(obj_id) => {
                        capturing.remove(&obj_id);
                        wirehose.node_capture_stop(obj_id);
                    }
                    _ => {}
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

    /// Call when a node's capture eligibility might have changed.
    fn on_node(&self, node: &Node) -> Option<Command> {
        self.capturing.as_ref()?;

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

        if self.capturing.as_ref()?.contains(&node.object_id) {
            return None;
        }

        self.start_capture_command(node)
    }

    /// Call when a node gets a new input link.
    fn on_link(&self, node: &Node) -> Option<Command> {
        self.capturing.as_ref()?;

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

        self.start_capture_command(node)
    }

    /// Call when a node's output positions have changed.
    fn on_positions_changed(&self, node: &Node) -> Option<Command> {
        if !self.capturing.as_ref()?.contains(&node.object_id) {
            return None;
        }

        self.start_capture_command(node)
    }

    /// Call when a node has no more input links.
    fn on_removed(&self, node: &Node) -> Option<Command> {
        self.capturing.as_ref()?;

        self.stop_capture_command(node)
    }

    fn start_capture_command(&self, node: &Node) -> Option<Command> {
        self.capturing.as_ref()?;

        let object_serial = node.props.object_serial()?;

        let capture_sink =
            node.props
                .media_class()
                .as_ref()
                .is_some_and(|media_class| {
                    media_class::is_sink(media_class)
                        || media_class::is_source(media_class)
                });

        Some(Command::NodeCaptureStart(
            node.object_id,
            *object_serial,
            capture_sink,
        ))
    }

    fn stop_capture_command(&self, node: &Node) -> Option<Command> {
        self.capturing.as_ref()?;

        Some(Command::NodeCaptureStop(node.object_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::mock;

    #[test]
    fn state_metadata_insert() {
        let mut state = State::default();
        let wirehose = mock::WirehoseHandle::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &wirehose,
            StateEvent::MetadataMetadataName {
                object_id,
                metadata_name: metadata_name.clone(),
            },
        );

        let metadata = state.metadatas.get(&object_id).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name.clone()));

        let metadata = state.get_metadata_by_name(&metadata_name).unwrap();
        assert_eq!(metadata.metadata_name, Some(metadata_name));
    }

    #[test]
    fn state_metadata_remove() {
        let mut state = State::default();
        let wirehose = mock::WirehoseHandle::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &wirehose,
            StateEvent::MetadataMetadataName {
                object_id,
                metadata_name: metadata_name.clone(),
            },
        );

        state.update(&wirehose, StateEvent::Removed { object_id });

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
        let wirehose = mock::WirehoseHandle::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &wirehose,
            StateEvent::MetadataMetadataName {
                object_id,
                metadata_name: metadata_name.clone(),
            },
        );

        let key = String::from("key");
        let value = String::from("value");

        state.update(
            &wirehose,
            StateEvent::MetadataProperty {
                object_id,
                subject: 0,
                key: Some(key.clone()),
                value: Some(value.clone()),
            },
        );
        state.update(
            &wirehose,
            StateEvent::MetadataProperty {
                object_id,
                subject: 1,
                key: Some(key.clone()),
                value: Some(value.clone()),
            },
        );
        assert_eq!(
            get_metadata_properties(&state, &object_id, 0).get(&key),
            Some(&value)
        );
        assert_eq!(
            get_metadata_properties(&state, &object_id, 1).get(&key),
            Some(&value)
        );

        state.update(
            &wirehose,
            StateEvent::MetadataProperty {
                object_id,
                subject: 0,
                key: Some(key.clone()),
                value: None,
            },
        );
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
        let wirehose = mock::WirehoseHandle::default();
        let object_id = ObjectId::from_raw_id(0);
        let metadata_name = String::from("metadata0");
        state.update(
            &wirehose,
            StateEvent::MetadataMetadataName {
                object_id,
                metadata_name: metadata_name.clone(),
            },
        );

        let key = String::from("key");
        let value = String::from("value");

        state.update(
            &wirehose,
            StateEvent::MetadataProperty {
                object_id,
                subject: 0,
                key: Some(key.clone()),
                value: Some(value.clone()),
            },
        );
        state.update(
            &wirehose,
            StateEvent::MetadataProperty {
                object_id,
                subject: 1,
                key: Some(key.clone()),
                value: Some(value.clone()),
            },
        );
        assert!(!get_metadata_properties(&state, &object_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &object_id, 1).is_empty());

        state.update(
            &wirehose,
            StateEvent::MetadataProperty {
                object_id,
                subject: 0,
                key: None,
                value: None,
            },
        );

        assert!(get_metadata_properties(&state, &object_id, 0).is_empty());
        assert!(!get_metadata_properties(&state, &object_id, 1).is_empty());
    }
}
