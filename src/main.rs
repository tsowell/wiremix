// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use anyhow::Result;
use clap::Parser;
use pipewire as pw;
use std::rc::Rc;
use std::{cell::RefCell, collections::HashMap};

use pw::{
    device::Device,
    loop_::Signal,
    node::Node,
    properties::properties,
    proxy::{Listener, ProxyListener, ProxyT},
    types::ObjectType,
};

use libspa::param::ParamType;
use libspa::pod::{
    deserialize::PodDeserializer, Object, Pod, Value, ValueArray,
};

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

fn node_props(id: u32, param: Object) {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PROP_channelVolumes {
            if let Value::ValueArray(ValueArray::Float(value)) = prop.value {
                if !value.is_empty() {
                    let mean = value.iter().sum::<f32>() / value.len() as f32;
                    let cubic = mean.cbrt();
                    println!("node {} {:?}", id, cubic);
                }
            }
        }
    }
}

fn device_route(id: u32, param: Object) {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                println!("device {} route index {}", id, value);
            }
        }
    }
}

fn device_enum_route(id: u32, param: Object) {
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

    let Some(index) = index else {
        return;
    };
    let Some(description) = description else {
        return;
    };

    println!("device {} route {}: {}", id, index, description);
}

fn device_profile(id: u32, param: Object) {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                println!("device {} profile index {:?}", id, value);
            }
        }
    }
}

fn device_enum_profile(id: u32, param: Object) {
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

    let Some(index) = index else {
        return;
    };
    let Some(description) = description else {
        return;
    };

    println!("device {} profile {}: {}", id, index, description);
}

fn monitor(remote: Option<String>) -> Result<()> {
    let main_loop = pw::main_loop::MainLoop::new(None)?;

    let main_loop_weak = main_loop.downgrade();
    let _sig_int =
        main_loop.loop_().add_signal_local(Signal::SIGINT, move || {
            if let Some(main_loop) = main_loop_weak.upgrade() {
                main_loop.quit();
            }
        });
    let main_loop_weak = main_loop.downgrade();
    let _sig_term =
        main_loop
            .loop_()
            .add_signal_local(Signal::SIGTERM, move || {
                if let Some(main_loop) = main_loop_weak.upgrade() {
                    main_loop.quit();
                }
            });

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

    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let obj_id = obj.id;
            if let Some(registry) = registry_weak.upgrade() {
                let p: Option<(Box<dyn ProxyT>, Box<dyn Listener>)> = match obj
                    .type_
                {
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
                            println!("node {} {}", obj_id, node_description);
                        }
                        let node: Node = registry.bind(obj).unwrap();
                        let obj_listener = node
                            .add_listener_local()
                            .param(move |_, id, _, _, param| {
                                if let Some(param) = deserialize(param) {
                                    match id {
                                        ParamType::Props => {
                                            node_props(obj_id, param)
                                        }
                                        _ => (),
                                    }
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
                            println!(
                                "device {} {}",
                                obj_id, device_description
                            );
                        }
                        let device: Device = registry.bind(obj).unwrap();
                        let obj_listener = device
                            .add_listener_local()
                            .param(move |_, id, _, _, param| {
                                if let Some(param) = deserialize(param) {
                                    match id {
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
                                        _ => (),
                                    }
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
            }
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
    pw::init();

    let opt = Opt::parse();
    monitor(opt.remote)?;

    unsafe {
        pw::deinit();
    }

    Ok(())
}
