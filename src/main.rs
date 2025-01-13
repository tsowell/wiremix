// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use anyhow::Result;
use clap::Parser;
use pipewire as pw;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;
use std::{cell::RefCell, collections::HashMap};

use pw::{
    device::Device,
    main_loop::{MainLoop, WeakMainLoop},
    node::Node,
    properties::properties,
    proxy::{Listener, ProxyListener, ProxyT},
    types::ObjectType,
};

use libspa::param::ParamType;
use libspa::pod::{
    deserialize::PodDeserializer, Object, Pod, Value, ValueArray,
};

#[allow(dead_code)]
#[derive(Debug)]
enum MonitorMessage {
    DeviceDescription(u32, String),
    NodeDescription(u32, String),
    DeviceRouteIndex(u32, i32),
    DeviceRouteDescription(u32, i32, String),
    DeviceProfileIndex(u32, i32),
    DeviceProfileDescription(u32, i32, String),
    NodeVolume(u32, f32),
    Removed(u32),
}

struct Proxies {
    proxies_t: HashMap<u32, Box<dyn ProxyT>>,
    listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
}

impl Proxies {
    fn new() -> Self {
        Self {
            proxies_t: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    fn add_proxy_t(
        &mut self,
        proxy_t: Box<dyn ProxyT>,
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

    fn add_proxy_listener(&mut self, proxy_id: u32, listener: ProxyListener) {
        let v = self.listeners.entry(proxy_id).or_default();
        v.push(Box::new(listener));
    }

    fn remove(&mut self, proxy_id: u32) {
        self.proxies_t.remove(&proxy_id);
        self.listeners.remove(&proxy_id);
    }
}

fn deserialize(param: Option<&Pod>) -> Option<Object> {
    param
        .and_then(|pod| {
            PodDeserializer::deserialize_any_from(pod.as_bytes()).ok()
        })
        .and_then(|(_, value)| match value {
            Value::Object(obj) => Some(obj),
            _ => None,
        })
}

fn node_props(id: u32, param: Object) -> Option<MonitorMessage> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PROP_channelVolumes {
            if let Value::ValueArray(ValueArray::Float(value)) = prop.value {
                if !value.is_empty() {
                    let mean = value.iter().sum::<f32>() / value.len() as f32;
                    let cubic = mean.cbrt();
                    return Some(MonitorMessage::NodeVolume(id, cubic));
                }
            }
        }
    }

    None
}

fn device_route(id: u32, param: Object) -> Option<MonitorMessage> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorMessage::DeviceRouteIndex(id, value));
            }
        }
    }

    None
}

fn device_enum_route(id: u32, param: Object) -> Option<MonitorMessage> {
    let mut index = None;
    let mut description = None;

    for prop in param.properties {
        match prop.key {
            libspa_sys::SPA_PARAM_ROUTE_index => {
                if let Value::Int(value) = prop.value {
                    index = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_description => {
                if let Value::String(value) = prop.value {
                    description = Some(value);
                }
            }
            _ => (),
        }
    }

    Some(MonitorMessage::DeviceRouteDescription(
        id,
        index?,
        description?,
    ))
}

fn device_profile(id: u32, param: Object) -> Option<MonitorMessage> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorMessage::DeviceProfileIndex(id, value));
            }
        }
    }

    None
}

fn device_enum_profile(id: u32, param: Object) -> Option<MonitorMessage> {
    let mut index = None;
    let mut description = None;

    for prop in param.properties {
        match prop.key {
            libspa_sys::SPA_PARAM_PROFILE_index => {
                if let Value::Int(value) = prop.value {
                    index = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_PROFILE_description => {
                if let Value::String(value) = prop.value {
                    description = Some(value);
                }
            }
            _ => (),
        }
    }

    Some(MonitorMessage::DeviceProfileDescription(
        id,
        index?,
        description?,
    ))
}

struct MessageSender {
    tx: mpsc::Sender<MonitorMessage>,
    main_loop_weak: WeakMainLoop,
}

impl MessageSender {
    fn new(
        tx: mpsc::Sender<MonitorMessage>,
        main_loop_weak: WeakMainLoop,
    ) -> Self {
        Self { tx, main_loop_weak }
    }

    fn send(&self, message: Option<MonitorMessage>) {
        if let Some(message) = message {
            if let Err(_) = self.tx.send(message) {
                if let Some(main_loop) = self.main_loop_weak.upgrade() {
                    main_loop.quit();
                }
            }
        }
    }
}

fn monitor(
    remote: Option<String>,
    tx: mpsc::Sender<MonitorMessage>,
) -> Result<()> {
    let main_loop = MainLoop::new(None)?;

    let context = pw::context::Context::new(&main_loop)?;
    let props = remote.map(|remote| {
        properties! {
            *pw::keys::REMOTE_NAME => remote
        }
    });
    let core = context.connect(props)?;

    let registry = Rc::new(core.get_registry()?);
    let registry_weak = Rc::downgrade(&registry);

    // Proxies and their listeners need to stay alive so store them here
    let proxies = Rc::new(RefCell::new(Proxies::new()));

    let sender = Rc::new(MessageSender::new(tx, main_loop.downgrade()));
    let remove_sender = Rc::clone(&sender);
    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let obj_id = obj.id;
            let Some(registry) = registry_weak.upgrade() else {
                return;
            };
            let p: Option<(Box<dyn ProxyT>, Box<dyn Listener>)> =
                match obj.type_ {
                    ObjectType::Node => {
                        let Some(props) = obj.props else { return };
                        let Some(media_class) = props.get("media.class") else {
                            return;
                        };
                        match media_class {
                            "Audio/Sink" => (),
                            "Audio/Source" => (),
                            "Stream/Output/Audio" => (),
                            _ => return,
                        }
                        if let Some(node_description) =
                            props.get("node.description")
                        {
                            let message = MonitorMessage::NodeDescription(
                                obj_id,
                                String::from(node_description),
                            );
                            sender.send(Some(message));
                        }
                        let node: Node = registry.bind(obj).unwrap();
                        let sender = Rc::clone(&sender);
                        let obj_listener = node
                            .add_listener_local()
                            .param(move |_, id, _, _, param| {
                                if let Some(param) = deserialize(param) {
                                    sender.send(match id {
                                        ParamType::Props => {
                                            node_props(obj_id, param)
                                        }
                                        _ => None,
                                    });
                                }
                            })
                            .register();
                        node.subscribe_params(&[ParamType::Props]);

                        Some((Box::new(node), Box::new(obj_listener)))
                    }
                    ObjectType::Device => {
                        let Some(props) = obj.props else { return };
                        let Some(media_class) = props.get("media.class") else {
                            return;
                        };
                        match media_class {
                            "Audio/Device" => (),
                            _ => return,
                        }
                        if let Some(device_description) =
                            props.get("device.description")
                        {
                            let message = MonitorMessage::DeviceDescription(
                                obj_id,
                                String::from(device_description),
                            );
                            sender.send(Some(message));
                        }
                        let device: Device = registry.bind(obj).unwrap();
                        let sender = Rc::clone(&sender);
                        let obj_listener = device
                            .add_listener_local()
                            .param(move |_, id, _, _, param| {
                                if let Some(param) = deserialize(param) {
                                    sender.send(match id {
                                        ParamType::Route => {
                                            device_route(obj_id, param)
                                        }
                                        ParamType::EnumRoute => {
                                            device_enum_route(obj_id, param)
                                        }
                                        ParamType::Profile => {
                                            device_profile(obj_id, param)
                                        }
                                        ParamType::EnumProfile => {
                                            device_enum_profile(obj_id, param)
                                        }
                                        _ => None,
                                    });
                                }
                            })
                            .register();
                        device.subscribe_params(&[
                            ParamType::Route,
                            ParamType::EnumRoute,
                            ParamType::Profile,
                            ParamType::EnumProfile,
                        ]);

                        Some((Box::new(device), Box::new(obj_listener)))
                    }
                    _ => None,
                };

            if let Some((proxy_spe, listener_spe)) = p {
                let proxy = proxy_spe.upcast_ref();
                let proxy_id = proxy.id();
                // Use a weak ref to prevent references cycle between Proxy and proxies:
                // - ref on proxies in the closure, bound to the Proxy lifetime
                // - proxies owning a ref on Proxy as well
                let proxies_weak = Rc::downgrade(&proxies);

                let listener = proxy
                    .add_listener_local()
                    .removed(move || {
                        if let Some(proxies) = proxies_weak.upgrade() {
                            proxies.borrow_mut().remove(proxy_id);
                        }
                    })
                    .register();

                proxies.borrow_mut().add_proxy_t(proxy_spe, listener_spe);
                proxies.borrow_mut().add_proxy_listener(proxy_id, listener);
            }
        })
        .global_remove(move |id| {
            remove_sender.send(Some(MonitorMessage::Removed(id)));
        })
        .register();

    main_loop.run();

    Ok(())
}

#[derive(Parser)]
#[clap(name = "pwmixer", about = "PipeWire mixer")]
struct Opt {
    #[clap(short, long, help = "The name of the remote to connect to")]
    remote: Option<String>,
}

fn main() -> Result<()> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        pw::init();

        let opt = Opt::parse();
        let _ = monitor(opt.remote, tx);

        unsafe {
            pw::deinit();
        }
    });

    for received in rx {
        println!("{:?}", received);
    }

    Ok(())
}
