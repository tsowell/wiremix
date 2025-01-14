mod device;
mod proxy;

use anyhow::Result;
use pipewire as pw;
use std::rc::Rc;
use std::sync::mpsc;
use std::cell::RefCell;

use pw::{
    device::Device,
    main_loop::{MainLoop, WeakMainLoop},
    node::Node,
    properties::properties,
    proxy::{Listener, ProxyT},
    registry::{GlobalObject, Registry},
    types::ObjectType,
};

use libspa::param::ParamType;
use libspa::pod::{
    deserialize::PodDeserializer, Object, Pod, Value, ValueArray,
};
use libspa::utils::dict::DictRef;

use crate::message::MonitorMessage;
use crate::monitor::device::DeviceStatusTracker;
use crate::monitor::proxy::Proxies;

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
            if self.tx.send(message).is_err() {
                if let Some(main_loop) = self.main_loop_weak.upgrade() {
                    main_loop.quit();
                }
            }
        }
    }
}

type ProxyInfo = (Box<Rc<dyn ProxyT>>, Box<dyn Listener>);

fn monitor_node(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<MessageSender>,
) -> Option<ProxyInfo> {
    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Sink" => (),
        "Audio/Source" => (),
        "Stream/Output/Audio" => (),
        _ => return None,
    }

    let node: Node = registry.bind(obj).ok()?;
    let node = Rc::new(node);
    let proxy_id = node.upcast_ref().id();

    if let Some(node_description) = props.get("node.description") {
        let message = MonitorMessage::NodeDescription(
            proxy_id,
            String::from(node_description),
        );
        sender.send(Some(message));
    }

    let sender = Rc::clone(sender);
    let listener = node
        .add_listener_local()
        .param(move |_seq, id, _index, _next, param| {
            if let Some(param) = deserialize(param) {
                sender.send(match id {
                    ParamType::Props => node_props(proxy_id, param),
                    _ => None,
                });
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props]);

    Some((Box::new(node), Box::new(listener)))
}

fn monitor_device(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<MessageSender>,
    statuses: &Rc<RefCell<DeviceStatusTracker>>,
) -> Option<ProxyInfo> {
    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Device" => (),
        _ => return None,
    }

    let device: Device = registry.bind(obj).ok()?;
    let device = Rc::new(device);
    let proxy_id = device.upcast_ref().id();

    if let Some(device_description) = props.get("device.description") {
        let message = MonitorMessage::DeviceDescription(
            proxy_id,
            String::from(device_description),
        );
        sender.send(Some(message));
    }

    let params = [
        ParamType::Route,
        ParamType::EnumRoute,
        ParamType::Profile,
        ParamType::EnumProfile,
    ];

    let sender = Rc::clone(sender);
    let listener = device
        .add_listener_local()
        .param({
            let statuses = Rc::clone(statuses);
            move |_seq, id, _index, _next, param| {
                if let Some(param) = deserialize(param) {
                    sender.send(match id {
                        ParamType::Route => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_route(proxy_id, param)
                        }
                        ParamType::EnumRoute => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_enum_route(proxy_id, param)
                        }
                        ParamType::Profile => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_profile(proxy_id, param)
                        }
                        ParamType::EnumProfile => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_enum_profile(proxy_id, param)
                        }
                        _ => None,
                    });
                }
            }
        })
        .info({
            let device = Rc::clone(&device);
            let statuses = Rc::clone(statuses);
            move |_info| {
                let statuses = statuses.borrow();
                let Some(status) = statuses.get(proxy_id) else {
                    return;
                };
                for param in params.into_iter() {
                    if !status.get(param) {
                        device.enum_params(0, Some(param), 0, u32::MAX);
                    }
                }
            }
        })
        .register();

    device.subscribe_params(&params);

    Some((Box::new(device), Box::new(listener)))
}

pub fn monitor_pipewire(
    remote: Option<String>,
    tx: mpsc::Sender<MonitorMessage>,
) -> Result<()> {
    pw::init();

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

    let statuses = Rc::new(RefCell::new(DeviceStatusTracker::new()));

    let sender = Rc::new(MessageSender::new(tx, main_loop.downgrade()));
    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let Some(registry) = registry_weak.upgrade() else {
                return;
            };
            let p: Option<ProxyInfo> = match obj.type_ {
                ObjectType::Node => monitor_node(&registry, obj, &sender),
                ObjectType::Device => {
                    monitor_device(&registry, obj, &sender, &statuses)
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

                let sender = Rc::clone(&sender);
                let listener = proxy
                    .add_listener_local()
                    .removed(move || {
                        if let Some(proxies) = proxies_weak.upgrade() {
                            proxies.borrow_mut().remove(proxy_id);
                            let message = MonitorMessage::Removed(proxy_id);
                            sender.send(Some(message));
                        }
                    })
                    .register();

                proxies.borrow_mut().add_proxy_t(proxy_spe, listener_spe);
                proxies.borrow_mut().add_proxy_listener(proxy_id, listener);
            }
        })
        .register();

    main_loop.run();

    unsafe {
        pw::deinit();
    }

    Ok(())
}
