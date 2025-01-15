use std::rc::Rc;

use pipewire::{
    node::{Node, NodeChangeMask, NodeInfoRef},
    proxy::ProxyT,
    registry::{GlobalObject, Registry},
};

use libspa::{
    param::ParamType,
    pod::{Object, Value, ValueArray},
    utils::dict::DictRef,
};

use crate::message::MonitorMessage;
use crate::monitor::{deserialize::deserialize, MessageSender, ProxyInfo};

pub fn monitor_node(
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
                        node_info_props(&sender, proxy_id, info);
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
                    if let Some(message) = match id {
                        ParamType::Props => node_param_props(proxy_id, param),
                        _ => None,
                    } {
                        sender.send(message);
                    }
                }
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props]);

    Some((Box::new(node), Box::new(listener)))
}

fn node_info_props(
    sender: &Rc<MessageSender>,
    id: u32,
    node_info: &NodeInfoRef,
) {
    let Some(props) = node_info.props() else {
        return;
    };

    if let Some(node_name) = props.get("node.name") {
        sender.send(MonitorMessage::NodeName(id, node_name.to_string()));
    }

    if let Some(node_nick) = props.get("node.nick") {
        sender.send(MonitorMessage::NodeNick(id, node_nick.to_string()));
    }

    if let Some(node_description) = props.get("node.description") {
        sender.send(MonitorMessage::NodeDescription(
            id,
            node_description.to_string(),
        ));
    }
}

fn node_param_props(id: u32, param: Object) -> Option<MonitorMessage> {
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
