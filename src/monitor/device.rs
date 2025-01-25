use std::cell::RefCell;
use std::rc::Rc;

use pipewire::{
    device::{Device, DeviceChangeMask, DeviceInfoRef},
    proxy::ProxyT,
    registry::{GlobalObject, Registry},
};

use libspa::{
    param::ParamType,
    pod::{Object, Value},
    utils::dict::DictRef,
};

use crate::event::MonitorEvent;
use crate::monitor::{
    deserialize::deserialize, device_status::DeviceStatusTracker, EventSender,
    ProxyInfo,
};
use crate::object::ObjectId;

pub fn monitor_device(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
    statuses: &Rc<RefCell<DeviceStatusTracker>>,
) -> Option<ProxyInfo> {
    let obj_id = ObjectId::from(obj);

    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Device" => (),
        _ => return None,
    }

    sender.send(MonitorEvent::DeviceMediaClass(
        obj_id,
        media_class.to_string(),
    ));

    let device: Device = registry.bind(obj).ok()?;
    let device = Rc::new(device);
    let proxy_id = device.upcast_ref().id();

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
                    if let Some(event) = match id {
                        ParamType::Route => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_route(obj_id, param)
                        }
                        ParamType::EnumRoute => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_enum_route(obj_id, param)
                        }
                        ParamType::Profile => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_profile(obj_id, param)
                        }
                        ParamType::EnumProfile => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_enum_profile(obj_id, param)
                        }
                        _ => None,
                    } {
                        sender.send(event);
                    }
                }
            }
        })
        .info({
            let sender_weak = Rc::downgrade(sender);
            let device_weak = Rc::downgrade(&device);
            let statuses_weak = Rc::downgrade(statuses);
            move |info| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                for change in info.change_mask().iter() {
                    if change == DeviceChangeMask::PROPS {
                        device_info_props(&sender, obj_id, info);
                    }
                }

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

fn device_route(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorEvent::DeviceRouteIndex(id, value));
            }
        }
    }

    None
}

fn device_enum_route(id: ObjectId, param: Object) -> Option<MonitorEvent> {
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

    Some(MonitorEvent::DeviceRouteDescription(
        id,
        index?,
        description?,
    ))
}

fn device_profile(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorEvent::DeviceProfileIndex(id, value));
            }
        }
    }

    None
}

fn device_enum_profile(id: ObjectId, param: Object) -> Option<MonitorEvent> {
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

    Some(MonitorEvent::DeviceProfileDescription(
        id,
        index?,
        description?,
    ))
}

fn device_info_props(
    sender: &Rc<EventSender>,
    id: ObjectId,
    device_info: &DeviceInfoRef,
) {
    let Some(props) = device_info.props() else {
        return;
    };

    if let Some(device_name) = props.get("device.name") {
        sender.send(MonitorEvent::DeviceName(id, device_name.to_string()));
    }

    if let Some(device_nick) = props.get("device.nick") {
        sender.send(MonitorEvent::DeviceNick(id, device_nick.to_string()));
    }

    if let Some(device_description) = props.get("device.description") {
        sender.send(MonitorEvent::DeviceDescription(
            id,
            device_description.to_string(),
        ));
    }
}
