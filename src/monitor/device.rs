use std::cell::RefCell;
use std::rc::Rc;

use pipewire::{
    device::Device,
    proxy::ProxyT,
    registry::{GlobalObject, Registry},
};

use libspa::{
    param::ParamType,
    pod::{Object, Value},
    utils::dict::DictRef,
};

use crate::message::MonitorMessage;
use crate::monitor::deserialize::deserialize;
use crate::monitor::device_status::DeviceStatusTracker;
use crate::monitor::MessageSender;
use crate::monitor::ProxyInfo;

pub fn monitor_device(
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
