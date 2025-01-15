mod deserialize;
mod device_status;
mod message_sender;
mod node;
mod proxy_registry;
mod stream;
mod stream_registry;

use anyhow::Result;
use pipewire as pw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use pw::{
    device::Device,
    main_loop::MainLoop,
    properties::properties,
    proxy::{Listener, ProxyT},
    registry::{GlobalObject, Registry},
    types::ObjectType,
};

use libspa::param::ParamType;
use libspa::pod::{Object, Value};
use libspa::utils::dict::DictRef;

use crate::message::MonitorMessage;
use crate::monitor::deserialize::deserialize;
use crate::monitor::device_status::DeviceStatusTracker;
use crate::monitor::message_sender::MessageSender;
use crate::monitor::proxy_registry::ProxyRegistry;
use crate::monitor::stream_registry::StreamRegistry;

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

type ProxyInfo = (Box<Rc<dyn ProxyT>>, Box<dyn Listener>);

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
        sender.send(message);
    }

    let params = [
        ParamType::Route,
        ParamType::EnumRoute,
        ParamType::Profile,
        ParamType::EnumProfile,
    ];

    let listener = device
        .add_listener_local()
        .param({
            let sender_weak = Rc::downgrade(sender);
            let statuses_weak = Rc::downgrade(statuses);
            move |_seq, id, _index, _next, param| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                let Some(statuses) = statuses_weak.upgrade() else {
                    return;
                };
                if let Some(param) = deserialize(param) {
                    if let Some(message) = match id {
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
                    } {
                        sender.send(message);
                    }
                }
            }
        })
        .info({
            let device_weak = Rc::downgrade(&device);
            let statuses_weak = Rc::downgrade(statuses);
            move |_info| {
                let Some(device) = device_weak.upgrade() else {
                    return;
                };
                let Some(statuses) = statuses_weak.upgrade() else {
                    return;
                };
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
    let proxies = Rc::new(RefCell::new(ProxyRegistry::new()));
    let streams = Rc::new(RefCell::new(StreamRegistry::new()));

    let statuses = Rc::new(RefCell::new(DeviceStatusTracker::new()));

    let sender = Rc::new(MessageSender::new(tx, main_loop.downgrade()));
    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let Some(registry) = registry_weak.upgrade() else {
                return;
            };
            let (p, s) = match obj.type_ {
                ObjectType::Node => {
                    let p = node::monitor_node(&registry, obj, &sender);
                    if let Some((ref proxy, _)) = p {
                        let id = proxy.upcast_ref().id();
                        (p, stream::capture_node(&core, obj, &sender, id))
                    } else {
                        (p, None)
                    }
                }
                ObjectType::Device => {
                    (monitor_device(&registry, obj, &sender, &statuses), None)
                }
                _ => (None, None),
            };

            let Some((proxy_spe, listener_spe)) = p else {
                return;
            };

            let proxy = proxy_spe.upcast_ref();
            let proxy_id = proxy.id();
            // Use a weak ref to prevent references cycle between Proxy and proxies:
            // - ref on proxies in the closure, bound to the Proxy lifetime
            // - proxies owning a ref on Proxy as well
            let proxies_weak = Rc::downgrade(&proxies);

            let stream_info = s.as_ref().map(|(stream_spe, _)| {
                (Rc::downgrade(&streams), Rc::downgrade(stream_spe))
            });
            let sender_weak = Rc::downgrade(&sender);
            let listener = proxy
                .add_listener_local()
                .removed(move || {
                    let Some(sender) = sender_weak.upgrade() else {
                        return;
                    };
                    let Some(proxies) = proxies_weak.upgrade() else {
                        return;
                    };

                    proxies.borrow_mut().remove(proxy_id);

                    sender.send(MonitorMessage::Removed(proxy_id));
                })
                .removed(move || {
                    let Some((ref streams_weak, ref stream_spe_weak)) =
                        stream_info
                    else {
                        return;
                    };
                    let Some(streams) = streams_weak.upgrade() else {
                        return;
                    };
                    let Some(stream_spe) = stream_spe_weak.upgrade() else {
                        return;
                    };

                    let _ = stream_spe.disconnect();
                    streams.borrow_mut().remove(proxy_id);
                })
                .register();

            let mut proxies = proxies.borrow_mut();
            proxies.add_proxy_t(proxy_spe, listener_spe);
            proxies.add_proxy_listener(proxy_id, listener);

            if let Some((stream_spe, listener_spe)) = s {
                let mut streams = streams.borrow_mut();
                streams.add_stream(proxy_id, stream_spe, listener_spe);
            }
        })
        .register();

    main_loop.run();

    unsafe {
        pw::deinit();
    }

    Ok(())
}
