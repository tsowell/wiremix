use std::rc::Rc;

use pipewire::{
    metadata::Metadata,
    proxy::Listener,
    registry::{GlobalObject, Registry},
};

use libspa::utils::dict::DictRef;

use crate::event::MonitorEvent;
use crate::monitor::{EventSender, ObjectId};

pub fn monitor_metadata(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<(Rc<Metadata>, Box<dyn Listener>)> {
    let obj_id = ObjectId::from(obj);

    let props = obj.props?;
    let metadata_name = props.get("metadata.name")?;
    if metadata_name != "default" {
        return None;
    }

    sender.send(MonitorEvent::MetadataMetadataName(
        obj_id,
        metadata_name.to_string(),
    ));

    let metadata: Metadata = registry.bind(obj).ok()?;
    let metadata = Rc::new(metadata);

    let listener = metadata
        .add_listener_local()
        .property({
            let sender_weak = Rc::downgrade(sender);
            move |subject, key, _type, value| {
                let Some(sender) = sender_weak.upgrade() else {
                    return 0;
                };

                sender.send(MonitorEvent::MetadataProperty(
                    obj_id,
                    subject,
                    key.map(str::to_string),
                    value.map(str::to_string),
                ));

                0
            }
        })
        .register();

    Some((metadata, Box::new(listener)))
}
