use std::rc::Rc;

use pipewire::{
    client::{Client, ClientChangeMask, ClientInfoRef},
    proxy::Listener,
    registry::{GlobalObject, Registry},
};

use libspa::utils::dict::DictRef;

use crate::event::MonitorEvent;
use crate::monitor::EventSender;
use crate::object::ObjectId;

pub fn monitor_client(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Client>, Box<dyn Listener>)> {
    let obj_id = ObjectId::from(obj);

    let client: Client = registry.bind(obj).ok()?;
    let client = Rc::new(client);

    let listener = client
        .add_listener_local()
        .info({
            let sender_weak = Rc::downgrade(sender);
            move |info| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                for change in info.change_mask().iter() {
                    if change == ClientChangeMask::PROPS {
                        client_info_props(&sender, obj_id, info);
                    }
                }
            }
        })
        .register();

    Some((client, Box::new(listener)))
}

fn client_info_props(
    sender: &EventSender,
    id: ObjectId,
    client_info: &ClientInfoRef,
) {
    let Some(props) = client_info.props() else {
        return;
    };

    if let Some(application_name) = props.get("application.name") {
        sender.send(MonitorEvent::ClientApplicationName(
            id,
            String::from(application_name),
        ));
    }

    if let Some(application_process_binary) =
        props.get("application.process.binary")
    {
        sender.send(MonitorEvent::ClientApplicationProcessBinary(
            id,
            String::from(application_process_binary),
        ));
    }
}
