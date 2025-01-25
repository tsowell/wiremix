use std::collections::HashMap;
use std::rc::Rc;

use pipewire::{
    device::Device,
    link::Link,
    metadata::Metadata,
    node::Node,
    proxy::{Listener, ProxyListener},
};

use crate::object::ObjectId;

pub struct ProxyRegistry {
    devices: HashMap<ObjectId, Rc<Device>>,
    nodes: HashMap<ObjectId, Rc<Node>>,
    links: HashMap<ObjectId, Rc<Link>>,
    metadatas: HashMap<ObjectId, Rc<Metadata>>,
    listeners: HashMap<ObjectId, Vec<Box<dyn Listener>>>,
}

impl ProxyRegistry {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
            nodes: HashMap::new(),
            links: HashMap::new(),
            metadatas: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    pub fn add_device(
        &mut self,
        obj_id: ObjectId,
        device: Rc<Device>,
        listener: Box<dyn Listener>,
    ) {
        self.devices.insert(obj_id, device);

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    pub fn add_node(
        &mut self,
        obj_id: ObjectId,
        node: Rc<Node>,
        listener: Box<dyn Listener>,
    ) {
        self.nodes.insert(obj_id, node);

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    pub fn add_link(
        &mut self,
        obj_id: ObjectId,
        link: Rc<Link>,
        listener: Box<dyn Listener>,
    ) {
        self.links.insert(obj_id, link);

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    pub fn add_metadata(
        &mut self,
        obj_id: ObjectId,
        metadata: Rc<Metadata>,
        listener: Box<dyn Listener>,
    ) {
        self.metadatas.insert(obj_id, metadata);

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    pub fn add_proxy_listener(
        &mut self,
        obj_id: ObjectId,
        listener: ProxyListener,
    ) {
        let v = self.listeners.entry(obj_id).or_default();
        v.push(Box::new(listener));
    }

    pub fn remove(&mut self, obj_id: ObjectId) {
        self.devices.remove(&obj_id);
        self.nodes.remove(&obj_id);
        self.links.remove(&obj_id);
        self.metadatas.remove(&obj_id);
        self.listeners.remove(&obj_id);
    }
}
