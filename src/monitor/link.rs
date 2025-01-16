use std::rc::Rc;

use pipewire::{
    link::{Link, LinkChangeMask, LinkInfoRef},
    registry::{GlobalObject, Registry},
};

use libspa::utils::dict::DictRef;

use crate::message::MonitorMessage;
use crate::monitor::{MessageSender, ProxyInfo};

pub fn monitor_link(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<MessageSender>,
) -> Option<ProxyInfo> {
    let link: Link = registry.bind(obj).ok()?;
    let link = Rc::new(link);

    let listener = link
        .add_listener_local()
        .info({
            let sender_weak = Rc::downgrade(sender);
            move |info| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                for change in info.change_mask().iter() {
                    if change == LinkChangeMask::PROPS {
                        link_info_props(&sender, info);
                    }
                }
            }
        })
        .register();

    Some((Box::new(link), Box::new(listener)))
}

fn link_info_props(sender: &Rc<MessageSender>, link_info: &LinkInfoRef) {
    // Ignore props and get the nodes directly from the link info.
    sender.send(MonitorMessage::from(link_info));
}
