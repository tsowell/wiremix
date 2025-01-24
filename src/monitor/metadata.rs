use std::rc::Rc;

use pipewire::{
    //link::{Link, LinkChangeMask, LinkInfoRef},
    metadata::Metadata,
    registry::{GlobalObject, Registry},
};

use libspa::utils::dict::DictRef;

use crate::event::MonitorEvent;
use crate::monitor::{EventSender, ObjectId, ProxyInfo};

pub fn monitor_metadata(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<EventSender>,
) -> Option<ProxyInfo> {
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
            move |_subject, key, _type, value| {
                let Some(sender) = sender_weak.upgrade() else {
                    return 0;
                };
                match key {
                    Some("default.audio.sink") => {}
                    Some("default.audio.source") => {}
                    None => sender.send(MonitorEvent::Removed(obj_id)),
                    _ => return 0,
                }
                let Some(key) = key else {
                    return 0;
                };

                sender.send(MonitorEvent::MetadataProperty(
                    obj_id,
                    key.to_string(),
                    value.map(str::to_string),
                ));

                0
            }
        })
        .register();

    Some((Box::new(metadata), Box::new(listener)))
}
