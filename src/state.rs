use std::collections::HashMap;

use crate::message::{MonitorMessage, ObjectId};

#[allow(dead_code)]
#[derive(Debug)]
struct Profile {
    index: i32,
    description: String,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Route {
    index: i32,
    description: String,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
struct Device {
    id: ObjectId,
    name: Option<String>,
    nick: Option<String>,
    description: Option<String>,
    profile_index: Option<i32>,
    profiles: HashMap<i32, Profile>,
    route_index: Option<i32>,
    routes: HashMap<i32, Route>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
struct Node {
    id: ObjectId,
    name: Option<String>,
    nick: Option<String>,
    description: Option<String>,
    media_name: Option<String>,
    volume: Option<f32>,
    peak: Option<f32>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct State {
    nodes: HashMap<ObjectId, Node>,
    devices: HashMap<ObjectId, Device>,
    links: HashMap<ObjectId, ObjectId>,
}

impl State {
    pub fn update(&mut self, message: MonitorMessage) {
        match message {
            MonitorMessage::DeviceName(id, name) => {
                self.device_entry(id).name = Some(name);
            }
            MonitorMessage::DeviceDescription(id, description) => {
                self.device_entry(id).description = Some(description);
            }
            MonitorMessage::DeviceNick(id, nick) => {
                self.device_entry(id).nick = Some(nick);
            }
            MonitorMessage::DeviceRouteIndex(id, index) => {
                self.device_entry(id).route_index = Some(index);
            }
            MonitorMessage::DeviceRouteDescription(id, index, description) => {
                self.device_entry(id)
                    .routes
                    .insert(index, Route { index, description });
            }
            MonitorMessage::DeviceProfileIndex(id, index) => {
                self.device_entry(id).profile_index = Some(index);
            }
            MonitorMessage::DeviceProfileDescription(
                id,
                index,
                description,
            ) => {
                self.device_entry(id)
                    .profiles
                    .insert(index, Profile { index, description });
            }
            MonitorMessage::NodeName(id, name) => {
                self.node_entry(id).name = Some(name);
            }
            MonitorMessage::NodeDescription(id, description) => {
                self.node_entry(id).description = Some(description);
            }
            MonitorMessage::NodeNick(id, nick) => {
                self.node_entry(id).nick = Some(nick);
            }
            MonitorMessage::NodeMediaName(id, media_name) => {
                self.node_entry(id).media_name = Some(media_name);
            }
            MonitorMessage::NodeVolume(id, volume) => {
                self.node_entry(id).volume = Some(volume);
            }
            MonitorMessage::NodePeak(id, peak) => {
                self.node_entry(id).peak = Some(peak);
            }
            MonitorMessage::Link(output, input) => {
                self.links.insert(output, input);
            }
            MonitorMessage::Removed(id) => {
                self.devices.remove(&id);
                self.nodes.remove(&id);
                self.links.remove(&id);
            }
            MonitorMessage::Reset => {
                self.devices.clear();
                self.nodes.clear();
                self.links.clear();
            }
        }
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
}
