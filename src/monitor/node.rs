use std::rc::Rc;

use pipewire::{
    node::{Node, NodeChangeMask, NodeInfoRef},
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

pub fn monitor_node(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Node>, Box<dyn Listener>)> {
    let obj_id = ObjectId::from(obj);

    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Sink" => (),
        "Audio/Source" => (),
        "Stream/Output/Audio" => (),
        "Stream/Input/Audio" => (),
        _ => return None,
    }

    // Don't monitor capture streams to avoid clutter.
    match props.get("node.name") {
        // We especially don't want to capture our own capture streams.
        Some("wiremix-capture") => return None,
        Some("PulseAudio Volume Control") => return None,
        Some("ncpamixer") => return None,
        _ => (),
    }

    sender.send(MonitorEvent::NodeMediaClass(
        obj_id,
        MediaClass::from(media_class),
    ));

    let node: Node = registry.bind(obj).ok()?;
    let node = Rc::new(node);

    let listener = node
        .add_listener_local()
        .info({
            let sender_weak = Rc::downgrade(sender);
            move |info| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                for change in info.change_mask().iter() {
                    if change == NodeChangeMask::PROPS {
                        node_info_props(&sender, obj_id, info);
                    }
                }
            }
        })
        .param({
            let sender_weak = Rc::downgrade(sender);
            move |_seq, id, _index, _next, param| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                if let Some(param) = deserialize(param) {
                    match id {
                        ParamType::Props => {
                            node_param_props(&sender, obj_id, param);
                        }
                        ParamType::PortConfig => {
                            node_param_port_config(&sender, obj_id, param);
                        }
                        _ => {}
                    }
                }
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props, ParamType::PortConfig]);

    Some((node, Box::new(listener)))
}

fn node_info_props(
    sender: &EventSender,
    id: ObjectId,
    node_info: &NodeInfoRef,
) {
    let Some(props) = node_info.props() else {
        return;
    };

    if let Some(node_name) = props.get("node.name") {
        sender.send(MonitorEvent::NodeName(id, String::from(node_name)));
    }

    if let Some(node_nick) = props.get("node.nick") {
        sender.send(MonitorEvent::NodeNick(id, String::from(node_nick)));
    }

    if let Some(node_description) = props.get("node.description") {
        sender.send(MonitorEvent::NodeDescription(
            id,
            String::from(node_description),
        ));
    }

    if let Some(media_name) = props.get("media.name") {
        sender.send(MonitorEvent::NodeMediaName(id, String::from(media_name)));
    }

    if let Some(device_id) = props.get("device.id") {
        if let Ok(device_id) = device_id.parse() {
            sender.send(MonitorEvent::NodeDeviceId(
                id,
                ObjectId::from_raw_id(device_id),
            ));
        }
    }

    if let Some(client_id) = props.get("client.id") {
        if let Ok(client_id) = client_id.parse() {
            sender.send(MonitorEvent::NodeClientId(
                id,
                ObjectId::from_raw_id(client_id),
            ));
        }
    }

    if let Some(object_serial) = props.get("object.serial") {
        if let Ok(object_serial) = object_serial.parse() {
            sender.send(MonitorEvent::NodeObjectSerial(id, object_serial));
        }
    }

    if let Some(card_profile_device) = props.get("card.profile.device") {
        if let Ok(card_profile_device) = card_profile_device.parse() {
            sender.send(MonitorEvent::NodeCardProfileDevice(
                id,
                card_profile_device,
            ));
        }
    }
}

fn node_param_props(sender: &EventSender, id: ObjectId, param: Object) {
    for prop in param.properties {
        match prop.key {
            libspa_sys::SPA_PROP_channelVolumes => {
                if let Value::ValueArray(ValueArray::Float(value)) = prop.value
                {
                    sender.send(MonitorEvent::NodeVolumes(id, value));
                }
            }
            libspa_sys::SPA_PROP_mute => {
                if let Value::Bool(value) = prop.value {
                    sender.send(MonitorEvent::NodeMute(id, value));
                }
            }
            _ => {}
        }
    }
}

fn node_param_port_config(sender: &EventSender, id: ObjectId, param: Object) {
    let Some(format_prop) = param
        .properties
        .into_iter()
        .find(|prop| prop.key == libspa_sys::SPA_PARAM_PORT_CONFIG_format)
    else {
        return;
    };

    let Value::Object(Object { properties, .. }) = format_prop.value else {
        return;
    };

    let Some(position_prop) = properties
        .into_iter()
        .find(|prop| prop.key == libspa_sys::SPA_FORMAT_AUDIO_position)
    else {
        return;
    };

    let Value::ValueArray(ValueArray::Id(value)) = position_prop.value else {
        return;
    };

    let positions = value.into_iter().map(|x| x.0).collect();
    sender.send(MonitorEvent::NodePositions(id, positions));
}
