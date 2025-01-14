use std::collections::HashMap;
use std::rc::Rc;

use pipewire::proxy::{Listener, ProxyListener, ProxyT};

pub struct Proxies {
    proxies_t: HashMap<u32, Box<Rc<dyn ProxyT>>>,
    listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
}

impl Proxies {
    pub fn new() -> Self {
        Self {
            proxies_t: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    pub fn add_proxy_t(
        &mut self,
        proxy_t: Box<Rc<dyn ProxyT>>,
        listener: Box<dyn Listener>,
    ) {
        let proxy_id = {
            let proxy = proxy_t.upcast_ref();
            proxy.id()
        };

        self.proxies_t.insert(proxy_id, proxy_t);

        let v = self.listeners.entry(proxy_id).or_default();
        v.push(listener);
    }

    pub fn add_proxy_listener(
        &mut self,
        proxy_id: u32,
        listener: ProxyListener,
    ) {
        let v = self.listeners.entry(proxy_id).or_default();
        v.push(Box::new(listener));
    }

    pub fn remove(&mut self, proxy_id: u32) {
        self.proxies_t.remove(&proxy_id);
        self.listeners.remove(&proxy_id);
    }
}
