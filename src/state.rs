use std::collections::HashMap;

use crate::message::{MonitorMessage, ObjectId};

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
    pub volumes: Option<Vec<f32>>,
    pub peaks: Option<Vec<f32>>,
    pub positions: Option<Vec<u32>>,
    pub device_id: Option<ObjectId>,
}

#[allow(dead_code)]
#[derive(Default, Debug)]
pub struct State {
    pub nodes: HashMap<ObjectId, Node>,
    pub devices: HashMap<ObjectId, Device>,
    pub links: HashMap<ObjectId, ObjectId>,
}

impl State {
    pub fn update(&mut self, message: MonitorMessage) {
        match message {
            MonitorMessage::DeviceDescription(id, description) => {
                self.device_entry(id).description = Some(description);
            }
            MonitorMessage::DeviceMediaClass(id, media_class) => {
                self.device_entry(id).media_class = Some(media_class);
            }
            MonitorMessage::DeviceName(id, name) => {
                self.device_entry(id).name = Some(name);
            }
            MonitorMessage::DeviceNick(id, nick) => {
                self.device_entry(id).nick = Some(nick);
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
            MonitorMessage::DeviceProfileIndex(id, index) => {
                self.device_entry(id).profile_index = Some(index);
            }
            MonitorMessage::DeviceRouteDescription(id, index, description) => {
                self.device_entry(id)
                    .routes
                    .insert(index, Route { index, description });
            }
            MonitorMessage::DeviceRouteIndex(id, index) => {
                self.device_entry(id).route_index = Some(index);
            }
            MonitorMessage::NodeDescription(id, description) => {
                self.node_entry(id).description = Some(description);
            }
            MonitorMessage::NodeDeviceId(id, device_id) => {
                self.node_entry(id).device_id = Some(device_id);
            }
            MonitorMessage::NodeMediaClass(id, media_class) => {
                self.node_entry(id).media_class = Some(media_class);
            }
            MonitorMessage::NodeMediaName(id, media_name) => {
                self.node_entry(id).media_name = Some(media_name);
            }
            MonitorMessage::NodeName(id, name) => {
                self.node_entry(id).name = Some(name);
            }
            MonitorMessage::NodeNick(id, nick) => {
                self.node_entry(id).nick = Some(nick);
            }
            MonitorMessage::NodePeaks(id, peaks) => {
                self.node_entry(id).peaks = Some(peaks);
            }
            MonitorMessage::NodePositions(id, positions) => {
                self.node_entry(id).positions = Some(positions);
            }
            MonitorMessage::NodeVolumes(id, volumes) => {
                self.node_entry(id).volumes = Some(volumes);
            }
            MonitorMessage::Link(output, input) => {
                self.links.insert(output, input);
            }
            MonitorMessage::Removed(id) => {
                self.devices.remove(&id);
                self.nodes.remove(&id);
                self.links.remove(&id);
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
