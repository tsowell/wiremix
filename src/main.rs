use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pipewire::{
    context::Context,
    core::Core,
    main_loop::MainLoop,
    node::Node,
    registry::{GlobalObject, Listener, Registry},
    types::ObjectType,
};

use libspa::utils::dict::DictRef;

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

        let listener = registry
            .borrow()
            .add_listener_local()
            .global(move |global| {
                let bound = Self::bind_node(&registry_bind.borrow(), global);
                if let Some(node) = bound {
                    nodes.borrow_mut().insert(global.id, node);
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

    fn bind_node(
        registry: &Registry,
        global: &GlobalObject<&DictRef>,
    ) -> Option<Node> {
        match global.type_ {
            ObjectType::Node => {
                if let Some(props) = &global.props {
                    if let Some(media_class) = props.get("media.class") {
                        if MEDIA_CLASSES.contains(&media_class) {
                            println!("{}: {:?}", media_class, global);
                            if let Ok(node) = registry.bind(global) {
                                return Some(node);
                            }
                        }
                    }
                }
            }
            _ => (),
        }

        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = PipewireListener::try_new()?;

    listener.run();

    Ok(())
}
