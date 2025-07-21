use std::rc::Rc;

use pipewire::{
    client::{Client, ClientChangeMask, ClientInfoRef},
    proxy::Listener,
    registry::{GlobalObject, Registry},
};

use libspa::utils::dict::DictRef;

use crate::wirehose::event_sender::EventSender;
use crate::wirehose::{ObjectId, PropertyStore, StateEvent};

pub fn monitor_client(
    registry: &Registry,
    object: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Client>, Box<dyn Listener>)> {
    let object_id = ObjectId::from(object);

    let client: Client = registry.bind(object).ok()?;
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
                        client_info_props(&sender, object_id, info);
                    }
                }
            }
        })
        .register();

    Some((client, Box::new(listener)))
}

fn client_info_props(
    sender: &EventSender,
    object_id: ObjectId,
    client_info: &ClientInfoRef,
) {
    let Some(props) = client_info.props() else {
        return;
    };

    let property_store = PropertyStore::from(props);
    sender.send(StateEvent::ClientProperties {
        object_id,
        props: property_store,
    });
}
