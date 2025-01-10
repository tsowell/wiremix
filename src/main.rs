use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use pipewire::{
    main_loop::MainLoop,
    context::Context,
    registry::Listener,
    core::Core,
    node::Node,
    types::ObjectType
};

const MEDIA_CLASSES: &[&str] = &[
    "Audio/Device",
    "Audio/Sink",
    "Audio/Source",
    "Stream/Output/Audio",
];

struct PipewireListener {
    mainloop: MainLoop,
    _context: Context,
    _core: Core,
    _listener: Listener,
}

impl PipewireListener {
    pub fn try_new() -> Result<Self, Box<dyn std::error::Error>> {
        let mainloop = MainLoop::new(None)?;
        let context = Context::new(&mainloop)?;
        let core = context.connect(None)?;
        let registry = Rc::new(RefCell::new(core.get_registry()?));
        let nodes = Rc::new(RefCell::new(HashMap::<u32, Node>::new()));

        let nodes_remove = Rc::clone(&nodes);
        let registry_bind = Rc::clone(&registry);

        let listener = registry.borrow()
            .add_listener_local()
            .global(move |global| {
                match global.type_ {
                    ObjectType::Node => {
                        if let Some(props) = &global.props {
                            if let Some(media_class) = props.get("media.class") {
                                if MEDIA_CLASSES.contains(&media_class) {
                                    println!("{}: {:?}", media_class, global);
                                    if let Ok(node) = registry_bind.borrow().bind(global) {
                                        nodes.borrow_mut().insert(global.id, node);
                                    }
                                }
                            }
                        }
                    },
                    _ => (),
                }
            })
            .global_remove(move |id| {
                if nodes_remove.borrow_mut().remove(&id).is_some() {
                    println!("Removed: {}", id);
                }
            })
            .register();

        Ok(Self {
            mainloop,
            _context: context,
            _core: core,
            _listener: listener,
        })
    }

    pub fn run(&self) {
        self.mainloop.run()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = PipewireListener::try_new()?;

    listener.run();

    Ok(())
}
