//! View representing PipeWire state in a convenient format for rendering.

use itertools::Itertools;
use std::collections::HashMap;

use serde_json::json;

use crate::command::Command;
use crate::config;
use crate::device_type::DeviceType;
use crate::media_class::MediaClass;
use crate::object::ObjectId;
use crate::state;

/// A view for transforming [`State`](`crate::state::State`) into a better
/// format for rendering.
///
/// This is done in only two ways:
///
/// 1. [`Self::from()`] creates a View from scratch from a provided State.
///
/// 2. [`Self::update_peaks()`] updates just the provided peaks in an existing
///    View.
///
/// [`Self::from()`] is a bit expensive, but doesn't happen very often after we
/// get the initial state from PipeWire. Peak updates happen very frequently
/// though, hence the optimization.
///
/// There are also functions like [`Self::mute()`] for returning
/// [`Command`](`crate::command::Command`)s which can be sent to the
/// [`monitor`](`crate::monitor`).
#[derive(Debug, Default)]
pub struct View {
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,

    pub nodes_all: Vec<ObjectId>,
    pub nodes_playback: Vec<ObjectId>,
    pub nodes_recording: Vec<ObjectId>,
    pub nodes_output: Vec<ObjectId>,
    pub nodes_input: Vec<ObjectId>,

    pub devices_all: Vec<ObjectId>,

    pub sinks: Vec<(Target, String)>,
    pub sources: Vec<(Target, String)>,

    pub default_sink: Option<Target>,
    pub default_source: Option<Target>,

    pub metadata_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Node(ObjectId),
    Route(ObjectId, i32, i32),
    Profile(ObjectId, i32),
    Default,
}

#[derive(Debug)]
pub struct Node {
    pub id: ObjectId,
    pub object_serial: i32,
    pub name: String,
    pub title: String,
    pub title_source_sink: Option<String>,
    pub media_class: MediaClass,

    pub routes: Option<Vec<(Target, String)>>,

    pub target_title: String,
    pub target: Option<Target>,

    pub volumes: Vec<f32>,
    pub mute: bool,

    pub peaks: Option<Vec<f32>>,
    pub positions: Option<Vec<u32>>,

    /// If this is a device/endpoint node, store the (device_id, route_index,
    /// card_device) here because they are needed for the
    /// [`DeviceVolume`](`crate::command::DeviceVolume`) and
    /// [`DeviceMute`](`crate::command::DeviceMute`) commands.
    pub device_info: Option<(ObjectId, i32, i32)>,

    pub is_default_sink: bool,
    pub is_default_source: bool,
}

#[derive(Debug)]
pub struct Device {
    pub id: ObjectId,
    pub object_serial: i32,
    pub title: String,

    pub profiles: Vec<(Target, String)>,

    pub target_title: String,
    pub target: Option<Target>,
}

#[derive(Debug, Clone, Copy)]
pub enum VolumeAdjustment {
    Relative(f32),
    Absolute(f32),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum NodeType {
    Playback,
    Recording,
    Output,
    Input,
    #[default]
    All,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum ListType {
    Node(NodeType),
    #[default]
    Device,
}

impl ListType {
    pub fn is_node(&self) -> bool {
        matches!(self, ListType::Node(_))
    }

    pub fn is_device(&self) -> bool {
        matches!(self, ListType::Device)
    }
}

/// Gets the potential Target::Routes for a device and media class.
/// These come from the EnumRoutes where profiles contains the active profile's
/// index, and devices contains at least one of the profile's devices for the
/// given media class.
fn route_targets(
    device: &state::Device,
    media_class: &MediaClass,
) -> Option<Vec<(Target, String)>> {
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
                let route_device =
                    route.devices.iter().find(|route_device| {
                        profile_devices.contains(route_device)
                    })?;
                let title = if route.available {
                    route.description.clone()
                } else {
                    format!("{} (unavailable)", route.description)
                };
                Some((
                    Target::Route(device.id, route.index, *route_device),
                    title,
                ))
            })
            .collect(),
    )
}

/// Get the active route for a device and card device.
/// This is the route on a device Node IF the route's profile matches the
/// device's current profile. Otherwise, there is no valid route.
fn active_route(
    device: &state::Device,
    card_device: i32,
) -> Option<&state::Route> {
    let profile_index = device.profile_index?;

    device
        .routes
        .get(&card_device)
        .filter(|route| route.profiles.contains(&profile_index))
}

impl Node {
    fn from(
        state: &state::State,
        names: &config::Names,
        sources: &[(Target, String)],
        sinks: &[(Target, String)],
        default_sink_name: &Option<String>,
        default_source_name: &Option<String>,
        node: &state::Node,
    ) -> Option<Node> {
        let id = node.id;

        let media_class = node.media_class.as_ref()?.clone();
        let title = names.resolve(state, node)?;

        // Nodes can represent either streams or devices.
        let (volumes, mute, device_info) =
            if let Some(device_id) = node.device_id {
                // Nodes for devices should get their volume and mute status
                // from the associated device's active route which is also used
                // for changing the volume and mute status.
                let device = state.devices.get(&device_id)?;
                let card_device = node.card_profile_device?;
                if let Some(route) = active_route(device, card_device) {
                    let route_index = route.index;
                    (
                        route.volumes.clone(),
                        route.mute,
                        Some((device_id, route_index, card_device)),
                    )
                } else {
                    (node.volumes.as_ref()?.clone(), node.mute?, None)
                }
            } else {
                // We can interact with a stream node's volume and mute status
                // directly.
                (node.volumes.as_ref()?.clone(), node.mute?, None)
            };

        let (routes, target, target_title) = if let Some(device_id) =
            node.device_id
        {
            // Targets for device nodes are routes for the associated device.
            let device = state.devices.get(&device_id)?;
            let card_device = node.card_profile_device?;

            let mut routes: Vec<_> =
                route_targets(device, &media_class).unwrap_or_default();
            routes.sort_by(|(_, a), (_, b)| a.cmp(b));
            let routes = routes;

            let (target, target_title) = match active_route(device, card_device)
            {
                Some(route) => {
                    let target_title = if route.available {
                        route.description.clone()
                    } else {
                        format!("{} (unavailable)", route.description)
                    };
                    (
                        Some(Target::Route(
                            device.id,
                            route.index,
                            card_device,
                        )),
                        target_title,
                    )
                }
                None => (None, String::from("No route selected")),
            };

            (Some(routes), target, target_title)
        } else if media_class.is_sink_input() {
            // Targets for output streams are sinks.
            let outputs = state.outputs(id);
            let sink = sinks.iter().find(|(target, _)| {
                matches!(target, Target::Node(sink_id)
                    if outputs.contains(sink_id))
            });
            let (target, target_title) = if !has_target(state, node.id) {
                (
                    Some(Target::Default),
                    sink.map(|(_, title)| title.clone())
                        .unwrap_or(String::from("No default")),
                )
            } else {
                (
                    sink.map(|&(target, _)| target),
                    sink.map(|(_, title)| title.clone()).unwrap_or_default(),
                )
            };
            (None, target, target_title)
        } else if media_class.is_source_output() {
            // Targets for input streams are sources.
            let inputs = state.inputs(id);
            let source = sources.iter().find(|(target, _)| {
                matches!(target, Target::Node(source_id)
                    if inputs.contains(source_id))
            });
            let (target, target_title) = if !has_target(state, node.id) {
                (
                    Some(Target::Default),
                    source
                        .map(|(_, title)| title.clone())
                        .unwrap_or(String::from("No default")),
                )
            } else {
                (
                    source.map(|&(target, _)| target),
                    source.map(|(_, title)| title.clone()).unwrap_or_default(),
                )
            };
            (None, target, target_title)
        } else {
            (None, None, String::from("No route selected"))
        };

        Some(Self {
            id,
            object_serial: node.object_serial?,
            name: node.name.as_ref()?.clone(),
            title,
            title_source_sink: node.media_name.clone(),
            media_class,
            routes,
            target,
            target_title,
            volumes,
            mute,
            peaks: node.peaks.clone(),
            positions: node.positions.clone(),
            device_info,
            is_default_sink: *default_sink_name == node.name,
            is_default_source: *default_source_name == node.name,
        })
    }
}

impl Device {
    fn from(
        state: &state::State,
        device: &state::Device,
        names: &config::Names,
    ) -> Option<Device> {
        let id = device.id;

        let title = names.resolve(state, device)?;

        let mut profiles: Vec<_> = device
            .profiles
            .values()
            .map(|profile| {
                let title = if profile.available {
                    profile.description.clone()
                } else {
                    format!("{} (unavailable)", profile.description)
                };
                (profile.index, title)
            })
            .collect();
        profiles.sort_by_key(|&(index, _)| index);
        let profiles = profiles
            .into_iter()
            .map(|(index, title)| (Target::Profile(id, index), title))
            .collect();

        let target_profile = device.profiles.get(&device.profile_index?)?;
        let target_title = if target_profile.available {
            target_profile.description.clone()
        } else {
            format!("{} (unavailable)", target_profile.description)
        };

        let target = Some(Target::Profile(id, device.profile_index?));

        Some(Device {
            id,
            object_serial: device.object_serial?,
            title,
            profiles,
            target_title,
            target,
        })
    }
}

fn default_for(state: &state::State, which: &str) -> Option<String> {
    let metadata = state.get_metadata_by_name("default")?;
    let json = metadata.properties.get(&0)?.get(which)?;
    let obj = serde_json::from_str::<serde_json::Value>(json).ok()?;
    Some(String::from(obj["name"].as_str()?))
}

fn target_node(state: &state::State, node_id: ObjectId) -> Option<i64> {
    let metadata = state.get_metadata_by_name("default")?;
    let json = metadata
        .properties
        .get(&node_id.into())?
        .get("target.node")?;
    serde_json::from_str(json).ok()
}

fn target_object(state: &state::State, node_id: ObjectId) -> Option<i64> {
    let metadata = state.get_metadata_by_name("default")?;
    let json = metadata
        .properties
        .get(&node_id.into())?
        .get("target.object")?;
    serde_json::from_str(json).ok()
}

fn has_target(state: &state::State, node_id: ObjectId) -> bool {
    match (target_node(state, node_id), target_object(state, node_id)) {
        (Some(node), _) if node != -1 => true,
        (_, Some(object)) if object != -1 => true,
        _ => false,
    }
}

impl View {
    /// Create a View from scratch from a provided State.
    pub fn from(state: &state::State, names: &config::Names) -> View {
        let default_sink_name = default_for(state, "default.audio.sink");
        let default_source_name = default_for(state, "default.audio.source");

        let default_sink =
            default_sink_name.as_ref().and_then(|default_sink_name| {
                state
                    .nodes
                    .values()
                    .find(|node| node.name.as_ref() == Some(default_sink_name))
                    .map(|node| Target::Node(node.id))
            });

        let default_source =
            default_source_name
                .as_ref()
                .and_then(|default_source_name| {
                    state
                        .nodes
                        .values()
                        .find(|node| {
                            node.name.as_ref() == Some(default_source_name)
                        })
                        .map(|node| Target::Node(node.id))
                });

        let mut sinks: Vec<_> = state
            .nodes
            .values()
            .filter_map(|node| {
                if node.media_class.as_ref()?.is_sink() {
                    Some((Target::Node(node.id), names.resolve(state, node)?))
                } else {
                    None
                }
            })
            .collect();
        sinks.sort_by(|(_, a), (_, b)| a.cmp(b));
        let sinks = sinks;

        let mut sources: Vec<_> = state
            .nodes
            .values()
            .filter_map(|node| {
                if node.media_class.as_ref()?.is_source() {
                    let title = names.resolve(state, node)?;
                    Some((Target::Node(node.id), title))
                } else if node.media_class.as_ref()?.is_sink() {
                    let title = names.resolve(state, node)?;
                    Some((
                        Target::Node(node.id),
                        format!("Monitor of {}", title),
                    ))
                } else {
                    None
                }
            })
            .collect();
        sources.sort_by(|(_, a), (_, b)| a.cmp(b));
        let sources = sources;

        let nodes: HashMap<ObjectId, Node> = state
            .nodes
            .values()
            .filter_map(|node| {
                Node::from(
                    state,
                    names,
                    &sources,
                    &sinks,
                    &default_sink_name,
                    &default_source_name,
                    node,
                )
            })
            .map(|node| (node.id, node))
            .collect();

        let devices: HashMap<ObjectId, Device> = state
            .devices
            .values()
            .filter_map(|device| Device::from(state, device, names))
            .map(|device| (device.id, device))
            .collect();

        let mut nodes_all = Vec::new();
        let mut nodes_playback = Vec::new();
        let mut nodes_recording = Vec::new();
        let mut nodes_output = Vec::new();
        let mut nodes_input = Vec::new();
        for (id, node) in
            nodes.iter().sorted_by_key(|(_, node)| node.object_serial)
        {
            nodes_all.push(*id);
            if node.media_class.is_sink_input() {
                nodes_playback.push(*id);
            }
            if node.media_class.is_source_output() {
                nodes_recording.push(*id);
            }
            if node.media_class.is_sink() {
                nodes_output.push(*id);
            }
            if node.media_class.is_source() {
                nodes_input.push(*id);
            }
        }
        let nodes_all = nodes_all;
        let nodes_playback = nodes_playback;
        let nodes_recording = nodes_recording;
        let nodes_output = nodes_output;
        let nodes_input = nodes_input;

        let devices_all = devices
            .iter()
            .sorted_by_key(|(_, device)| device.object_serial)
            .map(|(&id, _)| id)
            .collect();

        Self {
            nodes,
            devices,
            nodes_all,
            nodes_playback,
            nodes_recording,
            nodes_output,
            nodes_input,
            devices_all,
            sinks,
            sources,
            default_sink,
            default_source,
            metadata_id: state.metadatas_by_name.get("default").copied(),
        }
    }

    /// Update just the peaks of an existing State.
    pub fn update_peaks(&mut self, state: &state::State) {
        for state_node in state.nodes.values() {
            if let Some(node) = self.nodes.get_mut(&state_node.id) {
                match &state_node.peaks {
                    Some(peaks) => {
                        let peaks_ref = node.peaks.get_or_insert_default();
                        peaks_ref.resize(peaks.len(), 0.0);
                        peaks_ref.copy_from_slice(peaks);
                    }
                    _ => node.peaks = None,
                }
            }
        }
    }

    /// Returns a command for setting the provided node as the default
    /// source/sink, depending on device_type.
    pub fn set_default(
        &self,
        node_id: ObjectId,
        device_type: DeviceType,
    ) -> Option<Command> {
        let node = self.nodes.get(&node_id)?;
        let key = match device_type {
            DeviceType::Source => "default.configured.audio.source",
            DeviceType::Sink => "default.configured.audio.sink",
        };
        let metadata_id = self.metadata_id?;

        Some(Command::MetadataSetProperty(
            metadata_id,
            0,
            String::from(key),
            Some(String::from("Spa:String:JSON")),
            Some(json!({ "name": &node.name }).to_string()),
        ))
    }

    /// Returns a command for setting the provided node's target to the
    /// provided target.
    pub fn set_target(
        &self,
        node_id: ObjectId,
        target: Target,
    ) -> Vec<Command> {
        let Some(metadata_id) = self.metadata_id else {
            return Vec::new();
        };

        match target {
            Target::Default => {
                vec![
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        String::from("target.object"),
                        Some(String::from("Spa:Id")),
                        Some(String::from("-1")),
                    ),
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        String::from("target.node"),
                        Some(String::from("Spa:Id")),
                        Some(String::from("-1")),
                    ),
                ]
            }
            Target::Node(target_id) => {
                vec![
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        String::from("target.object"),
                        None,
                        None,
                    ),
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        String::from("target.node"),
                        Some(String::from("Spa:Id")),
                        Some(target_id.to_string()),
                    ),
                ]
            }
            Target::Route(device_id, route_index, route_device) => {
                vec![Command::DeviceSetRoute(
                    device_id,
                    route_index,
                    route_device,
                )]
            }
            Target::Profile(device_id, profile_index) => {
                vec![Command::DeviceSetProfile(device_id, profile_index)]
            }
        }
    }

    /// Returns a command for muting the provided node.
    pub fn mute(&self, node_id: ObjectId) -> Option<Command> {
        let node = self.nodes.get(&node_id)?;
        let mute = !node.mute;

        if let Some((device_id, route_index, route_device)) = node.device_info {
            Some(Command::DeviceMute(
                device_id,
                route_index,
                route_device,
                mute,
            ))
        } else {
            Some(Command::NodeMute(node_id, mute))
        }
    }

    /// Returns a command for changing the volume of the provided node.
    pub fn volume(
        &self,
        node_id: ObjectId,
        adjustment: VolumeAdjustment,
    ) -> Option<Command> {
        let node = self.nodes.get(&node_id)?;

        let mut volumes = node.volumes.clone();
        if volumes.is_empty() {
            return None;
        }
        match adjustment {
            VolumeAdjustment::Relative(delta) => {
                let avg = volumes.iter().sum::<f32>() / volumes.len() as f32;
                volumes.fill((avg.cbrt() + delta).max(0.0).powi(3));
            }
            VolumeAdjustment::Absolute(volume) => {
                volumes.fill(volume.max(0.0).powi(3));
            }
        }
        let volumes = volumes;

        if let Some((device_id, route_index, route_device)) = node.device_info {
            Some(Command::DeviceVolumes(
                device_id,
                route_index,
                route_device,
                volumes,
            ))
        } else {
            Some(Command::NodeVolumes(node_id, volumes))
        }
    }

    fn ids(&self, node_type: ListType) -> &[ObjectId] {
        match node_type {
            ListType::Node(NodeType::Playback) => &self.nodes_playback,
            ListType::Node(NodeType::Recording) => &self.nodes_recording,
            ListType::Node(NodeType::Output) => &self.nodes_output,
            ListType::Node(NodeType::Input) => &self.nodes_input,
            ListType::Node(NodeType::All) => &self.nodes_all,
            ListType::Device => &self.devices_all,
        }
    }

    /// Gets all the nodes without filtering.
    pub fn full_nodes(&self, node_type: NodeType) -> Vec<&Node> {
        let node_ids = self.ids(ListType::Node(node_type));
        node_ids
            .iter()
            .filter_map(|node_id| self.nodes.get(node_id))
            .collect()
    }

    /// Gets all the devices without filtering.
    pub fn full_devices(&self) -> Vec<&Device> {
        let device_ids = self.ids(ListType::Device);
        device_ids
            .iter()
            .filter_map(|device_id| self.devices.get(device_id))
            .collect()
    }

    /// Returns the next node in the list_type after a provided node.
    pub fn next_id(
        &self,
        list_type: ListType,
        object_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let objects = self.ids(list_type);
        let next_index = match object_id {
            Some(object_id) => objects
                .iter()
                .position(|&id| id == object_id)?
                .saturating_add(1),
            None => 0,
        };
        objects.get(next_index).copied()
    }

    /// Returns the previous node in the list_type before a provided node.
    pub fn previous_id(
        &self,
        list_type: ListType,
        object_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let objects = self.ids(list_type);
        let next_index = match object_id {
            Some(object_id) => objects
                .iter()
                .position(|&id| id == object_id)?
                .saturating_sub(1),
            None => 0,
        };
        objects.get(next_index).copied()
    }

    /// Returns the index in the list_type for the provided object.
    pub fn position(
        &self,
        list_type: ListType,
        object_id: ObjectId,
    ) -> Option<usize> {
        self.ids(list_type).iter().position(|&id| id == object_id)
    }

    /// Returns length of the list_type.
    pub fn len(&self, list_type: ListType) -> usize {
        self.ids(list_type).len()
    }

    /// Returns the possible targets for a node.
    pub fn node_targets(
        &self,
        node_id: ObjectId,
    ) -> Option<(Vec<(Target, String)>, usize)> {
        let node = self.nodes.get(&node_id)?;

        // Get the target list appropriate to the node type
        let (mut targets, default) = if let Some(routes) = &node.routes {
            (routes.clone(), None)
        } else if node.media_class.is_sink_input() {
            (self.sinks.clone(), self.default_sink)
        } else if node.media_class.is_source_output() {
            (self.sources.clone(), self.default_source)
        } else {
            (Vec::new(), None)
        };
        // Get and format the name of the default target
        let default_name = default
            .and_then(|default| {
                targets
                    .iter()
                    .find(|(target, _)| *target == default)
                    .map(|(_, name)| format!("Default: {}", name))
            })
            .unwrap_or(String::from("Default: No default"));
        // Sort targets by name
        targets.sort_by(|(_, a), (_, b)| a.cmp(b));
        // If the targets are nodes, add the default node to the top
        if node.media_class.is_sink_input()
            || node.media_class.is_source_output()
        {
            targets.insert(0, (Target::Default, default_name.clone()));
        };
        let targets = targets;

        // Get, for return, the position of the current target
        // Default to 0 if for some reason we can't find it
        let selected_position = node
            .target
            .and_then(|node_target| {
                targets
                    .iter()
                    .position(|&(target, _)| target == node_target)
            })
            .unwrap_or(0);

        Some((targets, selected_position))
    }

    /// Returns the possible targets for a device.
    pub fn device_targets(
        &self,
        device_id: ObjectId,
    ) -> Option<(Vec<(Target, String)>, usize)> {
        let device = self.devices.get(&device_id)?;

        let targets = device.profiles.clone();
        let selected_position = device
            .target
            .and_then(|device_target| {
                targets
                    .iter()
                    .position(|&(target, _)| target == device_target)
            })
            .unwrap_or(0);

        Some((targets, selected_position))
    }
}
