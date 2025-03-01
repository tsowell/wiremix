use std::rc::Rc;

use pipewire::{
    device::{Device, DeviceChangeMask, DeviceInfoRef},
    proxy::Listener,
    registry::{GlobalObject, Registry},
};

use libspa::{
    param::ParamType,
    pod::{Object, Value, ValueArray},
    utils::dict::DictRef,
};

use crate::event::MonitorEvent;
use crate::media_class::MediaClass;
use crate::monitor::{deserialize::deserialize, EventSender};
use crate::object::ObjectId;

pub fn monitor_device(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Device>, Box<dyn Listener>)> {
    let obj_id = ObjectId::from(obj);

    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Device" => (),
        _ => return None,
    }

    sender.send(MonitorEvent::DeviceMediaClass(
        obj_id,
        MediaClass::from(media_class),
    ));

    let device: Device = registry.bind(obj).ok()?;
    let device = Rc::new(device);

    let params = [
        ParamType::EnumRoute,
        ParamType::Route,
        ParamType::Profile,
        ParamType::EnumProfile,
    ];

    let listener = device
        .add_listener_local()
        .param({
            let sender_weak = Rc::downgrade(sender);
            move |_seq, id, _index, _next, param| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                if let Some(param) = deserialize(param) {
                    if let Some(event) = match id {
                        ParamType::EnumRoute => {
                            device_enum_route(obj_id, param)
                        }
                        ParamType::Route => device_route(obj_id, param),
                        ParamType::Profile => device_profile(obj_id, param),
                        ParamType::EnumProfile => {
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
                for param in params.into_iter() {
                    device.enum_params(0, Some(param), 0, u32::MAX);
                }
            }
        })
        .register();

    device.subscribe_params(&params);

    Some((device, Box::new(listener)))
}

fn device_enum_route(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    let mut index = None;
    let mut description = None;
    let mut available = None;
    let mut profiles = None;
    let mut devices = None;

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
            libspa_sys::SPA_PARAM_ROUTE_available => {
                if let Value::Id(libspa::utils::Id(value)) = prop.value {
                    available =
                        Some(value != libspa_sys::SPA_PARAM_AVAILABILITY_no);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_profiles => {
                if let Value::ValueArray(ValueArray::Int(value)) = prop.value {
                    profiles = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_devices => {
                if let Value::ValueArray(ValueArray::Int(value)) = prop.value {
                    devices = Some(value);
                }
            }
            _ => {}
        }
    }

    Some(MonitorEvent::DeviceEnumRoute(
        id,
        index?,
        description?,
        available?,
        profiles?,
        devices?,
    ))
}

fn device_route(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    let mut index = None;
    let mut device = None;
    let mut profile = None;
    let mut description = None;
    let mut available = None;
    let mut channel_volumes = None;
    let mut mute = None;

    for prop in param.properties {
        match prop.key {
            libspa_sys::SPA_PARAM_ROUTE_index => {
                if let Value::Int(value) = prop.value {
                    index = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_device => {
                if let Value::Int(value) = prop.value {
                    device = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_profile => {
                if let Value::Int(value) = prop.value {
                    profile = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_description => {
                if let Value::String(value) = prop.value {
                    description = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_available => {
                if let Value::Id(libspa::utils::Id(value)) = prop.value {
                    available =
                        Some(value != libspa_sys::SPA_PARAM_AVAILABILITY_no);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_props => {
                if let Value::Object(value) = prop.value {
                    for prop in value.properties {
                        match prop.key {
                            libspa_sys::SPA_PROP_channelVolumes => {
                                if let Value::ValueArray(ValueArray::Float(
                                    value,
                                )) = prop.value
                                {
                                    channel_volumes = Some(value);
                                }
                            }
                            libspa_sys::SPA_PROP_mute => {
                                if let Value::Bool(value) = prop.value {
                                    mute = Some(value);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Some(MonitorEvent::DeviceRoute(
        id,
        index?,
        device?,
        profile?,
        description?,
        available?,
        channel_volumes?,
        mute?,
    ))
}

fn device_profile(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorEvent::DeviceProfile(id, value));
            }
        }
    }

    None
}

fn parse_class(value: &Value) -> Option<(MediaClass, Vec<i32>)> {
    if let Value::Struct(class) = value {
        if let [Value::String(name), _, _, Value::ValueArray(ValueArray::Int(devices))] =
            class.as_slice()
        {
            return Some((MediaClass::from(name.as_str()), devices.clone()));
        }
    }

    None
}

fn device_enum_profile(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    let mut index = None;
    let mut description = None;
    let mut available = None;
    let mut classes = None;

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
            libspa_sys::SPA_PARAM_PROFILE_available => {
                if let Value::Id(libspa::utils::Id(value)) = prop.value {
                    available =
                        Some(value != libspa_sys::SPA_PARAM_AVAILABILITY_no);
                }
            }
            libspa_sys::SPA_PARAM_PROFILE_classes => {
                if let Value::Struct(classes_struct) = prop.value {
                    if let Some(Value::Int(_)) = classes_struct.first() {
                        classes = Some(Vec::new());
                        for class in classes_struct.iter().skip(1) {
                            if let Some(classes) = &mut classes {
                                classes.extend(parse_class(class));
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    Some(MonitorEvent::DeviceEnumProfile(
        id,
        index?,
        description?,
        available?,
        classes?,
    ))
}

fn device_info_props(
    sender: &EventSender,
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

    if let Some(object_serial) = props.get("object.serial") {
        if let Ok(object_serial) = object_serial.parse() {
            sender.send(MonitorEvent::DeviceObjectSerial(id, object_serial));
        }
    }
}
