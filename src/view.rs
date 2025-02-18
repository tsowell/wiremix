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

    pub sinks: Vec<(ObjectId, String, bool)>,
    pub sources: Vec<(ObjectId, String, bool)>,

    pub metadata_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Copy)]
pub enum Map {
    Sink(ObjectId),
    Source(ObjectId),
    Route(i32),
}

#[derive(Debug)]
pub struct Node {
    pub id: ObjectId,
    pub object_serial: i32,
    pub name: String,
    pub title: String,
    pub title_source_sink: Option<String>,
    pub media_class: MediaClass,

    pub routes: Vec<(i32, String)>,

    pub map_title: String,
    pub map: Map,
    pub is_map_default: bool,

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

#[derive(Debug, Clone, Copy)]
pub enum NodeType {
    Playback,
    Recording,
    Output,
    Input,
    All,
}

impl Node {
    pub fn from(
        state: &state::State,
        sources: &[(ObjectId, String, bool)],
        sinks: &[(ObjectId, String, bool)],
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
                let route_device = node.card_profile_device?;
                let route = device.routes.get(&route_device)?;
                let route_index = route.index;
                (
                    route.volumes.clone(),
                    route.mute,
                    Some((device_id, route_index, route_device)),
                )
            } else {
                (node.volumes.as_ref()?.clone(), node.mute?, None)
            };

        let media_class = node.media_class.as_ref()?.clone();
        let (routes, map, map_title, is_map_default) = if let Some(device_id) = node.device_id {
            let device = state.devices.get(&device_id)?;
            let route_device = node.card_profile_device?;
            let route = device.routes.get(&route_device)?;

            let mut routes: Vec<_> = device
                .enum_routes
                .values()
                .map(|route| (route.index, route.description.clone()))
                .collect();
            routes.sort_by(|(_, a), (_, b)| a.cmp(b));
            let routes = routes;

            Some((routes, Map::Route(route.index), route.description.clone(), false))
        } else if media_class.is_sink_input() {
            let outputs = state.outputs(id);
            let (sink_id, map_title, _) = sinks
                .iter()
                .find(|(sink_id, _, _)| outputs.contains(sink_id))?;
            let is_map_default = !has_target(state, node.id);
            Some((Default::default(), Map::Sink(*sink_id), map_title.clone(), is_map_default))
        } else if media_class.is_source_output() {
            let outputs = state.outputs(id);
            let (source_id, map_title, _) = sources
                .iter()
                .find(|(source_id, _, _)| outputs.contains(source_id))?;
            let is_map_default = !has_target(state, node.id);
            Some((
                Default::default(),
                Map::Source(*source_id),
                map_title.clone(),
                is_map_default,
            ))
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
            map,
            map_title,
            is_map_default,
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
    let json = metadata.properties.get(&node_id.into())?.get("target.node")?;
    serde_json::from_str(json).ok()
}

fn target_object(state: &state::State, node_id: ObjectId) -> Option<i64> {
    let metadata = state.get_metadata_by_name("default")?;
    let json = metadata.properties.get(&node_id.into())?.get("target.object")?;
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

        let mut sinks: Vec<_> = state
            .nodes
            .values()
            .filter_map(|node| {
                if node.media_class.as_ref()?.is_sink() {
                    let is_default = default_sink_name.is_some() && node.name == default_sink_name;
                    Some((node.id, node.description.as_ref()?.clone(), is_default))
                } else {
                    None
                }
            })
            .collect();
        sinks.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
        let sinks = sinks;

        let mut sources: Vec<_> = state
            .nodes
            .values()
            .filter_map(|node| {
                let is_default = default_source_name.is_some() && node.name == default_source_name;
                if node.media_class.as_ref()?.is_source() {
                    let description = node.description.as_ref()?.clone();
                    Some((node.id, description, is_default))
                } else if node.media_class.as_ref()?.is_sink() {
                    let description = node.description.as_ref()?.clone();
                    Some((node.id, format!("Monitor of {}", description), is_default))
                } else {
                    None
                }
            })
            .collect();
        sources.sort_by(|(_, a, _), (_, b, _)| a.cmp(b));
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

    pub fn prev_node_id(
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
        self.node_ids(node_type).iter().position(|&id| id == node_id)
    }

    pub fn nodes_len(&self, node_type: NodeType) -> usize {
        self.node_ids(node_type).len()
    }
}
