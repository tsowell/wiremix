use std::collections::HashMap;
use std::rc::Rc;

use pipewire::proxy::{Listener, ProxyListener, ProxyT};

use crate::object::ObjectId;

pub struct ProxyRegistry {
    proxies_t: HashMap<ObjectId, Box<Rc<dyn ProxyT>>>,
    listeners: HashMap<ObjectId, Vec<Box<dyn Listener>>>,
}

impl ProxyRegistry {
    pub fn new() -> Self {
        Self {
            proxies_t: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    pub fn add_proxy_t(
        &mut self,
        obj_id: ObjectId,
        proxy_t: Box<Rc<dyn ProxyT>>,
        listener: Box<dyn Listener>,
    ) {
        self.proxies_t.insert(obj_id, proxy_t);

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
        self.proxies_t.remove(&obj_id);
        self.listeners.remove(&obj_id);
    }
}
