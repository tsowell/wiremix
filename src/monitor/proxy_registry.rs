use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;

use nix::sys::eventfd::{EfdFlags, EventFd};

use pipewire::{
    device::Device,
    link::Link,
    metadata::Metadata,
    node::Node,
    proxy::{Listener, ProxyListener, ProxyT},
};

use crate::object::ObjectId;

/// Storage for keeping proxies and their listeners alive
pub struct ProxyRegistry {
    /// Storage for keeping devices alive
    pub devices: HashMap<ObjectId, Rc<Device>>,
    /// Storage for keeping nodes alive
    pub nodes: HashMap<ObjectId, Rc<Node>>,
    /// Storage for keeping metadata alive
    pub metadatas: HashMap<ObjectId, Rc<Metadata>>,
    /// Storage for keeping links alive
    links: HashMap<ObjectId, Rc<Link>>,
    /// Storage for keeping listeners alive
    listeners: HashMap<ObjectId, Vec<Box<dyn Listener>>>,
    /// Devices, nodes, links, and metadata pending deletion
    garbage_proxies_t: Vec<Rc<dyn ProxyT>>,
    /// Listeners pending deletion
    garbage_listeners: Vec<Box<dyn Listener>>,
    /// EventFd for signalling to [`crate::monitor`] that objects are pending
    /// deletion and that [`Self::collect_garbage()`] needs to be called
    gc_fd: EventFd,
}

impl Drop for ProxyRegistry {
    fn drop(&mut self) {
        // Drop listeners while their proxies are still alive.
        self.garbage_listeners.clear();
        self.listeners.clear();
    }
}

impl ProxyRegistry {
    pub fn try_new() -> Result<Self> {
        let gc_fd = EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK)?;
        Ok(Self {
            devices: HashMap::new(),
            nodes: HashMap::new(),
            links: HashMap::new(),
            metadatas: HashMap::new(),
            listeners: HashMap::new(),
            garbage_proxies_t: Default::default(),
            garbage_listeners: Default::default(),
            gc_fd,
        })
    }

    pub fn gc_fd(&self) -> &EventFd {
        &self.gc_fd
    }

    /// Clean up proxies and listeners pending deletion. It is unsafe to call
    /// this from within the PipeWire main loop!
    pub fn collect_garbage(&mut self) {
        self.garbage_listeners.clear();
        self.garbage_proxies_t.clear();
        let _ = self.gc_fd.read();
    }

    /// Register a device and its listener, evicting any with the same ID.
    pub fn add_device(
        &mut self,
        obj_id: ObjectId,
        device: Rc<Device>,
        listener: Box<dyn Listener>,
    ) {
        if let Some(old) = self.devices.insert(obj_id, device) {
            self.garbage_proxies_t.push(old);
            if let Some(listeners) = self.listeners.get_mut(&obj_id) {
                self.garbage_listeners.append(listeners);
            }
            let _ = self.gc_fd.arm();
        }

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    /// Register a node and its listener, evicting any with the same ID.
    pub fn add_node(
        &mut self,
        obj_id: ObjectId,
        node: Rc<Node>,
        listener: Box<dyn Listener>,
    ) {
        if let Some(old) = self.nodes.insert(obj_id, node) {
            self.garbage_proxies_t.push(old);
            if let Some(listeners) = self.listeners.get_mut(&obj_id) {
                self.garbage_listeners.append(listeners);
            }
            let _ = self.gc_fd.arm();
        }

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    /// Register a link and its listener, evicting any with the same ID.
    pub fn add_link(
        &mut self,
        obj_id: ObjectId,
        link: Rc<Link>,
        listener: Box<dyn Listener>,
    ) {
        if let Some(old) = self.links.insert(obj_id, link) {
            self.garbage_proxies_t.push(old);
            if let Some(listeners) = self.listeners.get_mut(&obj_id) {
                self.garbage_listeners.append(listeners);
            }
            let _ = self.gc_fd.arm();
        }

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    /// Register metadata and its listener, evicting any with the same ID.
    pub fn add_metadata(
        &mut self,
        obj_id: ObjectId,
        metadata: Rc<Metadata>,
        listener: Box<dyn Listener>,
    ) {
        if let Some(old) = self.metadatas.insert(obj_id, metadata) {
            self.garbage_proxies_t.push(old);
            if let Some(listeners) = self.listeners.get_mut(&obj_id) {
                self.garbage_listeners.append(listeners);
            }
            let _ = self.gc_fd.arm();
        }

        let v = self.listeners.entry(obj_id).or_default();
        v.push(listener);
    }

    /// Register a listener, evicting any with the same ID.
    pub fn add_proxy_listener(
        &mut self,
        obj_id: ObjectId,
        listener: ProxyListener,
    ) {
        let v = self.listeners.entry(obj_id).or_default();
        v.push(Box::new(listener));
    }

    /// Remove an object, defering deletion until [`Self::collect_garbage()`]
    /// is called.
    pub fn remove(&mut self, obj_id: ObjectId) {
        if let Some(listeners) = self.listeners.get_mut(&obj_id) {
            if !listeners.is_empty() {
                let _ = self.gc_fd.arm();
            }
            self.garbage_listeners.append(listeners);
        }
        if let Some(old) = self.devices.remove(&obj_id) {
            self.garbage_proxies_t.push(old);
            let _ = self.gc_fd.arm();
        }
        if let Some(old) = self.nodes.remove(&obj_id) {
            self.garbage_proxies_t.push(old);
            let _ = self.gc_fd.arm();
        }
        if let Some(old) = self.links.remove(&obj_id) {
            self.garbage_proxies_t.push(old);
            let _ = self.gc_fd.arm();
        }
        if let Some(old) = self.metadatas.remove(&obj_id) {
            self.garbage_proxies_t.push(old);
            let _ = self.gc_fd.arm();
        }
    }
}
