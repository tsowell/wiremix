use std::rc::Rc;

use pipewire::{
    node::{Node, NodeChangeMask, NodeInfoRef},
    registry::{GlobalObject, Registry},
};

use libspa::{
    param::ParamType,
    pod::{Object, Value, ValueArray},
    utils::dict::DictRef,
};

use crate::event::{MonitorEvent, ObjectId};
use crate::monitor::{deserialize::deserialize, EventSender, ProxyInfo};

pub fn monitor_node(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<ProxyInfo> {
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
        Some("pwmixer-capture") => return None,
        Some("PulseAudio Volume Control") => return None,
        Some("ncpamixer") => return None,
        _ => (),
    }

    sender.send(MonitorEvent::NodeMediaClass(
        obj_id,
        media_class.to_string(),
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
                    if let Some(event) = match id {
                        ParamType::Props => node_param_props(obj_id, param),
                        ParamType::PortConfig => {
                            node_param_port_config(obj_id, param)
                        }
                        _ => None,
                    } {
                        sender.send(event);
                    }
                }
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props, ParamType::PortConfig]);

    Some((Box::new(node), Box::new(listener)))
}

fn node_info_props(
    sender: &Rc<EventSender>,
    id: ObjectId,
    node_info: &NodeInfoRef,
) {
    let Some(props) = node_info.props() else {
        return;
    };

    if let Some(node_name) = props.get("node.name") {
        sender.send(MonitorEvent::NodeName(id, node_name.to_string()));
    }

    if let Some(node_nick) = props.get("node.nick") {
        sender.send(MonitorEvent::NodeNick(id, node_nick.to_string()));
    }

    if let Some(node_description) = props.get("node.description") {
        sender.send(MonitorEvent::NodeDescription(
            id,
            node_description.to_string(),
        ));
    }

    if let Some(media_name) = props.get("media.name") {
        sender.send(MonitorEvent::NodeMediaName(id, media_name.to_string()));
    }

    if let Some(device_id) = props.get("device.id") {
        if let Ok(device_id) = device_id.parse() {
            sender.send(MonitorEvent::NodeDeviceId(
                id,
                ObjectId::from_raw_id(device_id),
            ));
        }
    }
}

fn node_param_props(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PROP_channelVolumes {
            if let Value::ValueArray(ValueArray::Float(value)) = prop.value {
                return Some(MonitorEvent::NodeVolumes(id, value));
            }
        }
    }

    None
}

fn node_param_port_config(id: ObjectId, param: Object) -> Option<MonitorEvent> {
    let format_prop = param
        .properties
        .into_iter()
        .find(|prop| prop.key == libspa_sys::SPA_PARAM_PORT_CONFIG_format)?;

    let Value::Object(Object { properties, .. }) = format_prop.value else {
        return None;
    };

    let position_prop = properties
        .into_iter()
        .find(|prop| prop.key == libspa_sys::SPA_FORMAT_AUDIO_position)?;

    let Value::ValueArray(ValueArray::Id(value)) = position_prop.value else {
        return None;
    };

    let positions = value.into_iter().map(|x| x.0).collect();
    Some(MonitorEvent::NodePositions(id, positions))
}
