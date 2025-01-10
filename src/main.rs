use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use pipewire::{
    main_loop::MainLoop,
    context::Context,
    registry::GlobalObject,
    properties::Properties,
    types::ObjectType
};
use libspa::utils::dict::DictRef;

const MEDIA_CLASSES: &[&str] = &[
    "Audio/Device",
    "Audio/Sink",
    "Audio/Source",
    "Stream/Output/Audio",
];

#[derive(Default)]
struct GlobalListener {
    nodes: HashMap<u32, GlobalObject<Properties>>,
}

impl GlobalListener {
    fn global(&mut self, global: &GlobalObject<&DictRef>) {
        match global.type_ {
            ObjectType::Node => {
                if let Some(props) = &global.props {
                    if let Some(media_class) = props.get("media.class") {
                        if MEDIA_CLASSES.contains(&media_class) {
                            println!("{}: {:?}", media_class, global);
                            self.nodes.insert(global.id, global.to_owned());
                        }
                    }
                }
            },
            _ => (),
        }
    }

    fn global_remove(&mut self, id: u32) {
        if self.nodes.remove(&id).is_some() {
            println!("Removed: {}", id);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;
    let global_listener = Rc::new(RefCell::new(GlobalListener::default()));
    let global_remove_listener = Rc::clone(&global_listener);

    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            global_listener.borrow_mut().global(global)
        })
        .global_remove(move |id| {
            global_remove_listener.borrow_mut().global_remove(id)
        })
        .register();

    mainloop.run();

    Ok(())
}
