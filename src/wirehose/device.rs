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

use crate::wirehose::event_sender::EventSender;
use crate::wirehose::{
    deserialize::deserialize, ObjectId, PropertyStore, StateEvent,
};

pub fn monitor_device(
    registry: &Registry,
    object: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Device>, Box<dyn Listener>)> {
    let object_id = ObjectId::from(object);

    let props = object.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Device" => (),
        _ => return None,
    }

    let device: Device = registry.bind(object).ok()?;
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
                            device_enum_route(object_id, param)
                        }
                        ParamType::Route => device_route(object_id, param),
                        ParamType::Profile => device_profile(object_id, param),
                        ParamType::EnumProfile => {
                            device_enum_profile(object_id, param)
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
                        device_info_props(&sender, object_id, info);
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

fn device_enum_route(object_id: ObjectId, param: Object) -> Option<StateEvent> {
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

    Some(StateEvent::DeviceEnumRoute {
        object_id,
        index: index?,
        description: description?,
        available: available?,
        profiles: profiles?,
        devices: devices?,
    })
}

fn device_route(object_id: ObjectId, param: Object) -> Option<StateEvent> {
    let mut index = None;
    let mut device = None;
    let mut profiles = None;
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
            libspa_sys::SPA_PARAM_ROUTE_profiles => {
                if let Value::ValueArray(ValueArray::Int(value)) = prop.value {
                    profiles = Some(value);
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

    Some(StateEvent::DeviceRoute {
        object_id,
        index: index?,
        device: device?,
        profiles: profiles?,
        description: description?,
        available: available?,
        channel_volumes: channel_volumes?,
        mute: mute?,
    })
}

fn device_profile(object_id: ObjectId, param: Object) -> Option<StateEvent> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(StateEvent::DeviceProfile {
                    object_id,
                    index: value,
                });
            }
        }
    }

    None
}

fn parse_class(value: &Value) -> Option<(String, Vec<i32>)> {
    if let Value::Struct(class) = value {
        if let [Value::String(name), _, _, Value::ValueArray(ValueArray::Int(devices))] =
            class.as_slice()
        {
            return Some((name.clone(), devices.clone()));
        }
    }

    None
}

fn device_enum_profile(
    object_id: ObjectId,
    param: Object,
) -> Option<StateEvent> {
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
                    // Usually the first element is the size, which we skip.
                    let skip = match classes_struct.first() {
                        Some(Value::Int(_)) => 1,
                        _ => 0,
                    };
                    classes = Some(Vec::new());
                    for class in classes_struct.iter().skip(skip) {
                        if let Some(classes) = &mut classes {
                            classes.extend(parse_class(class));
                        }
                    }
                }
            }
            _ => (),
        }
    }

    Some(StateEvent::DeviceEnumProfile {
        object_id,
        index: index?,
        description: description?,
        available: available?,
        classes: classes?,
    })
}

fn device_info_props(
    sender: &EventSender,
    object_id: ObjectId,
    device_info: &DeviceInfoRef,
) {
    let Some(props) = device_info.props() else {
        return;
    };

    let props = PropertyStore::from(props);
    sender.send(StateEvent::DeviceProperties { object_id, props });
}
