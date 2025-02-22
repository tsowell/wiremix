use itertools::Itertools;
use std::collections::HashMap;

use serde_json::json;

use crate::command::Command;
use crate::device_type::DeviceType;
use crate::media_class::MediaClass;
use crate::object::ObjectId;
use crate::state;

#[derive(Debug, Default)]
pub struct View {
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,

    pub nodes_all: Vec<ObjectId>,
    pub nodes_playback: Vec<ObjectId>,
    pub nodes_recording: Vec<ObjectId>,
    pub nodes_output: Vec<ObjectId>,
    pub nodes_input: Vec<ObjectId>,

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

    pub device_info: Option<(ObjectId, i32, i32)>,

    pub is_default_sink: bool,
    pub is_default_source: bool,
}

#[derive(Debug)]
pub struct Device {
    pub id: ObjectId,
}

#[derive(Debug, Clone, Copy)]
pub enum VolumeAdjustment {
    Relative(f32),
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
                Some((
                    Target::Route(device.id, route.index, *route_device),
                    route.description.clone(),
                ))
            })
            .collect(),
    )
}

/// Get the active route for a device and card device.
/// This is the route on a device Node IF the route's profile matches the
/// device's current profile. Otherwise, there is no valid route.
fn active_route<'a>(
    device: &'a state::Device,
    card_device: i32,
) -> Option<&'a state::Route> {
    let profile_index = device.profile_index?;

    device
        .routes
        .get(&card_device)
        .filter(|route| route.profile == profile_index)
}

impl Node {
    pub fn from(
        state: &state::State,
        sources: &[(Target, String)],
        sinks: &[(Target, String)],
        default_sink_name: &Option<String>,
        default_source_name: &Option<String>,
        node: &state::Node,
    ) -> Option<Node> {
        let id = node.id;

        let title = match (&node.description, &node.name, &node.media_name) {
            (_, Some(name), Some(media_name)) => {
                Some(format!("{name}: {media_name}"))
            }
            (Some(description), _, _) => Some(description.clone()),
            _ => None,
        }?;

        let (volumes, mute, device_info) =
            if let Some(device_id) = node.device_id {
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
                (node.volumes.as_ref()?.clone(), node.mute?, None)
            };

        let media_class = node.media_class.as_ref()?.clone();
        let (routes, target, target_title) = if let Some(device_id) =
            node.device_id
        {
            let device = state.devices.get(&device_id)?;
            let card_device = node.card_profile_device?;

            let mut routes: Vec<_> =
                route_targets(device, &media_class).unwrap_or_default();
            routes.sort_by(|(_, a), (_, b)| a.cmp(b));
            let routes = routes;

            let (target, target_title) = match active_route(device, card_device)
            {
                Some(route) => (
                    Some(Target::Route(device.id, route.index, card_device)),
                    route.description.clone(),
                ),
                None => (None, String::new()),
            };

            Some((Some(routes), target, target_title))
        } else if media_class.is_sink_input() {
            let outputs = state.outputs(id);
            let sink = sinks.iter().find(|(target, _)| {
                matches!(target, Target::Node(sink_id)
                    if outputs.contains(sink_id))
            });
            let (target, target_title) = if !has_target(state, node.id) {
                (
                    Some(Target::Default),
                    sink.map(|(_, title)| title.clone())
                        .unwrap_or("No default".to_string()),
                )
            } else {
                (
                    sink.map(|&(target, _)| target),
                    sink.map(|(_, title)| title.clone()).unwrap_or_default(),
                )
            };
            Some((None, target, target_title))
        } else if media_class.is_source_output() {
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
                        .unwrap_or("No default".to_string()),
                )
            } else {
                (
                    source.map(|&(target, _)| target),
                    source.map(|(_, title)| title.clone()).unwrap_or_default(),
                )
            };
            Some((None, target, target_title))
        } else {
            None
        }?;

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

fn default_for(state: &state::State, which: &str) -> Option<String> {
    let metadata = state.get_metadata_by_name("default")?;
    let json = metadata.properties.get(&0)?.get(which)?;
    let obj = serde_json::from_str::<serde_json::Value>(json).ok()?;
    Some(obj["name"].as_str()?.to_string())
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
    pub fn from(state: &state::State) -> View {
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
                    Some((
                        Target::Node(node.id),
                        node.description.as_ref()?.clone(),
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
                if node.media_class.as_ref()?.is_source() {
                    let description = node.description.as_ref()?.clone();
                    Some((Target::Node(node.id), description))
                } else if node.media_class.as_ref()?.is_sink() {
                    let description = node.description.as_ref()?.clone();
                    Some((
                        Target::Node(node.id),
                        format!("Monitor of {}", description),
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
                    &sources,
                    &sinks,
                    &default_sink_name,
                    &default_source_name,
                    node,
                )
            })
            .map(|node| (node.id, node))
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

        Self {
            nodes,
            devices: Default::default(),
            nodes_all,
            nodes_playback,
            nodes_recording,
            nodes_output,
            nodes_input,
            sinks,
            sources,
            default_sink,
            default_source,
            metadata_id: state.metadatas_by_name.get("default").copied(),
        }
    }

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

    pub fn set_target(
        &self,
        node_id: ObjectId,
        target: Target,
    ) -> Vec<Command> {
        let Some(metadata_id) = self.metadata_id else {
            return Default::default();
        };

        match target {
            Target::Default => {
                vec![
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        "target.object".to_string(),
                        Some("Spa:Id".to_string()),
                        Some("-1".to_string()),
                    ),
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        "target.node".to_string(),
                        Some("Spa:Id".to_string()),
                        Some("-1".to_string()),
                    ),
                ]
            }
            Target::Node(target_id) => {
                vec![
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        "target.object".to_string(),
                        None,
                        None,
                    ),
                    Command::MetadataSetProperty(
                        metadata_id,
                        node_id.into(),
                        "target.node".to_string(),
                        Some("Spa:Id".to_string()),
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
        }
    }

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

    fn node_ids(&self, node_type: NodeType) -> &[ObjectId] {
        match node_type {
            NodeType::Playback => &self.nodes_playback,
            NodeType::Recording => &self.nodes_recording,
            NodeType::Output => &self.nodes_output,
            NodeType::Input => &self.nodes_input,
            NodeType::All => &self.nodes_all,
        }
    }

    pub fn full_nodes(&self, node_type: NodeType) -> Vec<&Node> {
        let node_ids = self.node_ids(node_type);
        node_ids
            .iter()
            .filter_map(|node_id| self.nodes.get(node_id))
            .collect()
    }

    pub fn next_node_id(
        &self,
        node_type: NodeType,
        node_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let nodes = self.node_ids(node_type);
        let next_index = match node_id {
            Some(node_id) => nodes
                .iter()
                .position(|&id| id == node_id)?
                .saturating_add(1),
            None => 0,
        };
        nodes.get(next_index).copied()
    }

    pub fn previous_node_id(
        &self,
        node_type: NodeType,
        node_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let nodes = self.node_ids(node_type);
        let next_index = match node_id {
            Some(node_id) => nodes
                .iter()
                .position(|&id| id == node_id)?
                .saturating_sub(1),
            None => 0,
        };
        nodes.get(next_index).copied()
    }

    pub fn node_position(
        &self,
        node_type: NodeType,
        node_id: ObjectId,
    ) -> Option<usize> {
        self.node_ids(node_type)
            .iter()
            .position(|&id| id == node_id)
    }

    pub fn nodes_len(&self, node_type: NodeType) -> usize {
        self.node_ids(node_type).len()
    }

    pub fn targets(
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
            (Default::default(), None)
        };
        // Get and format the name of the default target
        let default_name = default
            .and_then(|default| {
                targets
                    .iter()
                    .find(|(target, _)| *target == default)
                    .map(|(_, name)| format!("Default: {}", name))
            })
            .unwrap_or("Default: No default".to_string());
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
}
