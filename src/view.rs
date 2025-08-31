//! View representing PipeWire state in a convenient format for rendering.

use itertools::Itertools;
use std::collections::HashMap;

use serde_json::json;

use crate::config;
use crate::device_kind::DeviceKind;
use crate::wirehose::{media_class, state, CommandSender, ObjectId};

/// A view for transforming [`State`](`state::State`) into a better format for
/// rendering.
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
/// There are also functions like [`Self::mute()`] for executing commands
/// against [`wirehose`](`crate::wirehose`).
pub struct View<'a> {
    wirehose: &'a dyn CommandSender,
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

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Target {
    Node(ObjectId),
    Route(ObjectId, i32, i32),
    Profile(ObjectId, i32),
    Default,
}

#[derive(Debug)]
pub struct Node {
    pub object_id: ObjectId,
    pub object_serial: u64,
    pub name: String,
    pub title: String,
    pub title_source_sink: Option<String>,
    pub media_class: String,

    pub routes: Option<Vec<(Target, String)>>,

    pub target_title: String,
    pub target: Option<Target>,

    pub volumes: Vec<f32>,
    pub mute: bool,

    pub peaks: Option<Vec<f32>>,
    pub positions: Option<Vec<u32>>,

    /// If this is a device/endpoint node, store the (device_id, route_index,
    /// card_device) here because they are needed for changing volumes and
    /// muting via [`wirehose`](`crate::wirehose`).
    pub device_info: Option<(ObjectId, i32, i32)>,

    pub is_default_sink: bool,
    pub is_default_source: bool,
}

#[derive(Debug)]
pub struct Device {
    pub object_id: ObjectId,
    pub object_serial: u64,
    pub title: String,

    pub profiles: Vec<(Target, String)>,

    pub target_title: String,
    pub target: Option<Target>,
}

#[derive(Debug, Clone, Copy)]
pub enum VolumeAdjustment {
    Relative(f32),
    Absolute(f32),
    RelativeBalance(f32),
    AbsoluteBalance(f32),
}

#[derive(Default, Debug, Clone, Copy)]
pub enum NodeKind {
    Playback,
    Recording,
    Output,
    Input,
    #[default]
    All,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum ListKind {
    Node(NodeKind),
    #[default]
    Device,
}

impl ListKind {
    pub fn is_node(&self) -> bool {
        matches!(self, ListKind::Node(_))
    }

    pub fn is_device(&self) -> bool {
        matches!(self, ListKind::Device)
    }
}

/// Gets the potential Target::Routes for a device and media class.
/// These come from the EnumRoutes where profiles contains the active profile's
/// index, and devices contains at least one of the profile's devices for the
/// given media class.
fn route_targets(
    device: &state::Device,
    media_class: &String,
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
                    Target::Route(device.object_id, route.index, *route_device),
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
        let object_id = node.object_id;

        let media_class = node.props.media_class()?.clone();
        let title = names.resolve(state, node)?;

        // Nodes can represent either streams or devices.
        let (volumes, mute, device_info) =
            if let Some(device_id) = node.props.device_id() {
                // Nodes for devices should get their volume and mute status
                // from the associated device's active route which is also used
                // for changing the volume and mute status.
                let device = state.devices.get(device_id)?;
                let card_device = *node.props.card_profile_device()?;
                if let Some(route) = active_route(device, card_device) {
                    let route_index = route.index;
                    (
                        route.volumes.clone(),
                        route.mute,
                        Some((*device_id, route_index, card_device)),
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
            node.props.device_id()
        {
            // Targets for device nodes are routes for the associated device.
            let device = state.devices.get(device_id)?;
            let card_device = *node.props.card_profile_device()?;

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
                            device.object_id,
                            route.index,
                            card_device,
                        )),
                        target_title,
                    )
                }
                None => (None, String::from("No route selected")),
            };

            (Some(routes), target, target_title)
        } else if media_class::is_sink_input(&media_class) {
            // Targets for output streams are sinks.
            let outputs = state.outputs(object_id);
            let sink = sinks.iter().find(|(target, _)| {
                matches!(target, Target::Node(sink_id)
                    if outputs.contains(sink_id))
            });
            let (target, target_title) = if !has_target(state, node.object_id) {
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
        } else if media_class::is_source_output(&media_class) {
            // Targets for input streams are sources.
            let inputs = state.inputs(object_id);
            let source = sources.iter().find(|(target, _)| {
                matches!(target, Target::Node(source_id)
                    if inputs.contains(source_id))
            });
            let (target, target_title) = if !has_target(state, node.object_id) {
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
            object_id,
            object_serial: *node.props.object_serial()?,
            name: node.props.node_name()?.clone(),
            title,
            title_source_sink: node.props.media_name().cloned(),
            media_class,
            routes,
            target,
            target_title,
            volumes,
            mute,
            peaks: node.peaks.clone(),
            positions: node.positions.clone(),
            device_info,
            is_default_sink: default_sink_name.as_ref()
                == node.props.node_name(),
            is_default_source: default_source_name.as_ref()
                == node.props.node_name(),
        })
    }
}

impl Device {
    fn from(
        state: &state::State,
        device: &state::Device,
        names: &config::Names,
    ) -> Option<Device> {
        let object_id = device.object_id;

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
            .map(|(index, title)| (Target::Profile(object_id, index), title))
            .collect();

        let target_profile = device.profiles.get(&device.profile_index?)?;
        let target_title = if target_profile.available {
            target_profile.description.clone()
        } else {
            format!("{} (unavailable)", target_profile.description)
        };

        let target = Some(Target::Profile(object_id, device.profile_index?));

        let object_serial = *device.props.object_serial()?;

        Some(Device {
            object_id,
            object_serial,
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
    let object = serde_json::from_str::<serde_json::Value>(json).ok()?;
    Some(String::from(object["name"].as_str()?))
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

impl<'a> View<'a> {
    pub fn new(wirehose: &'a dyn CommandSender) -> View<'a> {
        Self {
            wirehose,
            nodes: Default::default(),
            devices: Default::default(),
            nodes_all: Default::default(),
            nodes_playback: Default::default(),
            nodes_recording: Default::default(),
            nodes_output: Default::default(),
            nodes_input: Default::default(),
            devices_all: Default::default(),
            sinks: Default::default(),
            sources: Default::default(),
            default_sink: Default::default(),
            default_source: Default::default(),
            metadata_id: Default::default(),
        }
    }

    /// Create a View from scratch from a provided State.
    pub fn from(
        wirehose: &'a dyn CommandSender,
        state: &state::State,
        names: &config::Names,
    ) -> View<'a> {
        let default_sink_name = default_for(state, "default.audio.sink");
        let default_source_name = default_for(state, "default.audio.source");

        let default_sink =
            default_sink_name.as_ref().and_then(|default_sink_name| {
                state
                    .nodes
                    .values()
                    .find(|node| {
                        node.props.node_name() == Some(default_sink_name)
                    })
                    .map(|node| Target::Node(node.object_id))
            });

        let default_source =
            default_source_name
                .as_ref()
                .and_then(|default_source_name| {
                    state
                        .nodes
                        .values()
                        .find(|node| {
                            node.props.node_name() == Some(default_source_name)
                        })
                        .map(|node| Target::Node(node.object_id))
                });

        let mut sinks: Vec<_> = state
            .nodes
            .values()
            .filter_map(|node| {
                if media_class::is_sink(node.props.media_class()?) {
                    Some((
                        Target::Node(node.object_id),
                        names.resolve(state, node)?,
                    ))
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
                if media_class::is_source(node.props.media_class()?) {
                    let title = names.resolve(state, node)?;
                    Some((Target::Node(node.object_id), title))
                } else if media_class::is_sink(node.props.media_class()?) {
                    let title = names.resolve(state, node)?;
                    Some((
                        Target::Node(node.object_id),
                        format!("Monitor of {title}"),
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
            .map(|node| (node.object_id, node))
            .collect();

        let devices: HashMap<ObjectId, Device> = state
            .devices
            .values()
            .filter_map(|device| Device::from(state, device, names))
            .map(|device| (device.object_id, device))
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
            if media_class::is_sink_input(&node.media_class) {
                nodes_playback.push(*id);
            }
            if media_class::is_source_output(&node.media_class) {
                nodes_recording.push(*id);
            }
            if media_class::is_sink(&node.media_class) {
                nodes_output.push(*id);
            }
            if media_class::is_source(&node.media_class) {
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
            wirehose,
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
            if let Some(node) = self.nodes.get_mut(&state_node.object_id) {
                match &state_node.peaks {
                    Some(peaks) => {
                        let peaks_ref =
                            node.peaks.get_or_insert_with(Default::default);
                        peaks_ref.resize(peaks.len(), 0.0);
                        peaks_ref.copy_from_slice(peaks);
                    }
                    _ => node.peaks = None,
                }
            }
        }
    }

    /// Sets the provided node as the default source/sink, depending on
    /// device_kind.
    pub fn set_default(&self, node_id: ObjectId, device_kind: DeviceKind) {
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };
        let Some(metadata_id) = self.metadata_id else {
            return;
        };

        let key = match device_kind {
            DeviceKind::Source => "default.configured.audio.source",
            DeviceKind::Sink => "default.configured.audio.sink",
        };

        self.wirehose.metadata_set_property(
            metadata_id,
            0,
            String::from(key),
            Some(String::from("Spa:String:JSON")),
            Some(json!({ "name": &node.name }).to_string()),
        );
    }

    /// Sets the provided node's target to the provided target.
    pub fn set_target(&self, node_id: ObjectId, target: Target) {
        let Some(metadata_id) = self.metadata_id else {
            return;
        };

        match target {
            Target::Default => {
                self.wirehose.metadata_set_property(
                    metadata_id,
                    node_id.into(),
                    String::from("target.object"),
                    Some(String::from("Spa:Id")),
                    Some(String::from("-1")),
                );
                self.wirehose.metadata_set_property(
                    metadata_id,
                    node_id.into(),
                    String::from("target.node"),
                    Some(String::from("Spa:Id")),
                    Some(String::from("-1")),
                );
            }
            Target::Node(target_id) => {
                self.wirehose.metadata_set_property(
                    metadata_id,
                    node_id.into(),
                    String::from("target.object"),
                    None,
                    None,
                );
                self.wirehose.metadata_set_property(
                    metadata_id,
                    node_id.into(),
                    String::from("target.node"),
                    Some(String::from("Spa:Id")),
                    Some(target_id.to_string()),
                );
            }
            Target::Route(device_id, route_index, route_device) => {
                self.wirehose.device_set_route(
                    device_id,
                    route_index,
                    route_device,
                );
            }
            Target::Profile(device_id, profile_index) => {
                self.wirehose.device_set_profile(device_id, profile_index);
            }
        }
    }

    /// Mutes the provided node.
    pub fn mute(&self, node_id: ObjectId) {
        let Some(node) = self.nodes.get(&node_id) else {
            return;
        };

        let mute = !node.mute;

        if let Some((device_id, route_index, route_device)) = node.device_info {
            self.wirehose.device_mute(
                device_id,
                route_index,
                route_device,
                mute,
            );
        } else {
            self.wirehose.node_mute(node_id, mute);
        }
    }

    /// Get current balance (stereo only)
    fn balance(&self, volumes: &Vec<f32>) -> Option<f32> {
        if volumes.len() == 2 {
            Some((volumes[1] / volumes[0]) - 1.0)
        } else {
            None
        }
    }

    /// Update channel balance balance (stereo only)
    fn rebalance(&self, volumes: &mut Vec<f32>, balance: f32) {
        if let Some(bal) = self.balance(volumes) {
            let bal_new = balance.clamp(-1.0, 1.0);
            if bal <= 0.0 {
                volumes[1] = volumes[0] * (bal_new + 1.0);
            } else {
                volumes[0] = volumes[1] / (bal_new + 1.0);
            }
        }
    }

    /// Changes the volume of the provided node. If max volume is provided,
    /// won't change volume if result would be greater than max. Returns true
    /// if volume was changed, otherwise false.
    pub fn volume(
        &self,
        node_id: ObjectId,
        adjustment: VolumeAdjustment,
        max: Option<f32>,
    ) -> bool {
        let Some(node) = self.nodes.get(&node_id) else {
            return false;
        };

        let mut volumes = node.volumes.clone();
        if volumes.is_empty() {
            return false;
        }
        match adjustment {
            VolumeAdjustment::Relative(delta) => {
                let avg = volumes.iter().sum::<f32>() / volumes.len() as f32;
                volumes.fill((avg.cbrt() + delta).max(0.0).powi(3));
            }
            VolumeAdjustment::Absolute(volume) => {
                volumes.fill(volume.max(0.0).powi(3));
            }
            VolumeAdjustment::AbsoluteBalance(balance) => {
                self.rebalance(&mut volumes, balance);
            }
            VolumeAdjustment::RelativeBalance(delta) => {
                if let Some(balance) = self.balance(&volumes) {
                    self.rebalance(&mut volumes, balance + delta);
                }
            }
        }
        let volumes = volumes;

        if let Some(max) = max {
            if volumes
                .iter()
                .any(|volume| (volume.cbrt() * 100.0).round() > max)
            {
                return false;
            }
        }

        if let Some((device_id, route_index, route_device)) = node.device_info {
            self.wirehose.device_volumes(
                device_id,
                route_index,
                route_device,
                volumes,
            );
        } else {
            self.wirehose.node_volumes(node_id, volumes);
        }

        true
    }

    fn object_ids(&self, node_kind: ListKind) -> &[ObjectId] {
        match node_kind {
            ListKind::Node(NodeKind::Playback) => &self.nodes_playback,
            ListKind::Node(NodeKind::Recording) => &self.nodes_recording,
            ListKind::Node(NodeKind::Output) => &self.nodes_output,
            ListKind::Node(NodeKind::Input) => &self.nodes_input,
            ListKind::Node(NodeKind::All) => &self.nodes_all,
            ListKind::Device => &self.devices_all,
        }
    }

    /// Gets all the nodes without filtering.
    pub fn full_nodes(&self, node_kind: NodeKind) -> Vec<&Node> {
        let node_ids = self.object_ids(ListKind::Node(node_kind));
        node_ids
            .iter()
            .filter_map(|node_id| self.nodes.get(node_id))
            .collect()
    }

    /// Gets all the devices without filtering.
    pub fn full_devices(&self) -> Vec<&Device> {
        let device_ids = self.object_ids(ListKind::Device);
        device_ids
            .iter()
            .filter_map(|device_id| self.devices.get(device_id))
            .collect()
    }

    /// Returns the next node in the list_kind after a provided node.
    pub fn next_id(
        &self,
        list_kind: ListKind,
        object_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let objects = self.object_ids(list_kind);
        let next_index = match object_id {
            Some(object_id) => objects
                .iter()
                .position(|&id| id == object_id)?
                .saturating_add(1),
            None => 0,
        };
        objects.get(next_index).copied()
    }

    /// Returns the previous node in the list_kind before a provided node.
    pub fn previous_id(
        &self,
        list_kind: ListKind,
        object_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let objects = self.object_ids(list_kind);
        let next_index = match object_id {
            Some(object_id) => objects
                .iter()
                .position(|&id| id == object_id)?
                .saturating_sub(1),
            None => 0,
        };
        objects.get(next_index).copied()
    }

    /// Returns the index in the list_kind for the provided object.
    pub fn position(
        &self,
        list_kind: ListKind,
        object_id: ObjectId,
    ) -> Option<usize> {
        self.object_ids(list_kind)
            .iter()
            .position(|&id| id == object_id)
    }

    /// Returns length of the list_kind.
    pub fn len(&self, list_kind: ListKind) -> usize {
        self.object_ids(list_kind).len()
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
        } else if media_class::is_sink_input(&node.media_class) {
            (self.sinks.clone(), self.default_sink)
        } else if media_class::is_source_output(&node.media_class) {
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
                    .map(|(_, name)| format!("Default: {name}"))
            })
            .unwrap_or(String::from("Default: No default"));
        // Sort targets by name
        targets.sort_by(|(_, a), (_, b)| a.cmp(b));
        // If the targets are nodes, add the default node to the top
        if media_class::is_sink_input(&node.media_class)
            || media_class::is_source_output(&node.media_class)
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
