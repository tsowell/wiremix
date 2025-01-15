use std::rc::Rc;

use pipewire::node::Node;
use pipewire::registry::{GlobalObject, Registry};

use libspa::param::ParamType;
use libspa::pod::{Object, Value, ValueArray};
use pipewire::proxy::ProxyT;
use libspa::utils::dict::DictRef;

use crate::message::MonitorMessage;
use crate::monitor::deserialize::deserialize;
use crate::monitor::MessageSender;
use crate::monitor::ProxyInfo;

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

    if let Some(node_description) = props.get("node.description") {
        let message = MonitorMessage::NodeDescription(
            proxy_id,
            String::from(node_description),
        );
        sender.send(message);
    }

    let listener = node
        .add_listener_local()
        .param({
            let sender_weak = Rc::downgrade(sender);
            move |_seq, id, _index, _next, param| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                if let Some(param) = deserialize(param) {
                    if let Some(message) = match id {
                        ParamType::Props => node_props(proxy_id, param),
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
