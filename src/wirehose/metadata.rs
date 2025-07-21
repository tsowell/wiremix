use std::rc::Rc;

use pipewire::{
    metadata::Metadata,
    proxy::Listener,
    registry::{GlobalObject, Registry},
};

use libspa::utils::dict::DictRef;

use crate::wirehose::event_sender::EventSender;
use crate::wirehose::{ObjectId, StateEvent};

pub fn monitor_metadata(
    registry: &Registry,
    object: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Metadata>, Box<dyn Listener>)> {
    let object_id = ObjectId::from(object);

    let props = object.props?;
    let metadata_name = props.get("metadata.name")?;
    if metadata_name != "default" {
        return None;
    }

    sender.send(StateEvent::MetadataMetadataName {
        object_id,
        metadata_name: String::from(metadata_name),
    });

    let metadata: Metadata = registry.bind(object).ok()?;
    let metadata = Rc::new(metadata);

    let listener = metadata
        .add_listener_local()
        .property({
            let sender_weak = Rc::downgrade(sender);
            move |subject, key, _type, value| {
                let Some(sender) = sender_weak.upgrade() else {
                    return 0;
                };

                sender.send(StateEvent::MetadataProperty {
                    object_id,
                    subject,
                    key: key.map(String::from),
                    value: value.map(String::from),
                });

                0
            }
        })
        .register();

    Some((metadata, Box::new(listener)))
}
