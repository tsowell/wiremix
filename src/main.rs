use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use pipewire::{
    context::Context,
    core::Core,
    main_loop::MainLoop,
    node::{Node, NodeListener},
    registry::{GlobalObject, Listener, Registry},
    types::ObjectType,
};

use libspa_sys;

use libspa::param::ParamType;
use libspa::pod::{deserialize::PodDeserializer, Pod};
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
        let nodes =
            Rc::new(RefCell::new(HashMap::<u32, (Node, NodeListener)>::new()));

        let nodes_remove = Rc::clone(&nodes);
        let registry_bind = Rc::clone(&registry);

        let listener = registry
            .borrow()
            .add_listener_local()
            .global(move |global| {
                let bound = Self::bind_node(&registry_bind.borrow(), global);
                if let Some(node) = bound {
                    println!("{:?}", global);
                    let listener = node
                        .add_listener_local()
                        .param(Self::node_listen_volume)
                        .register();
                    node.subscribe_params(&[ParamType::Props]);
                    nodes.borrow_mut().insert(global.id, (node, listener));
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
        if global.type_ != ObjectType::Node {
            return None;
        }

        let props = global.props?;
        let media_class = props.get("media.class")?;
        if !MEDIA_CLASSES.contains(&media_class) {
            return None;
        }

        registry.bind(global).ok()
    }

    fn node_listen_volume(
        _seq: i32,
        id: ParamType,
        _index: u32,
        _next: u32,
        param: Option<&Pod>,
    ) {
        fn pod_get_volume(param: &Pod) -> Option<Vec<f32>> {
            let obj = param.as_object().ok()?;
            let prop = obj
                .props()
                .find(|p| p.key().0 == libspa_sys::SPA_PROP_channelVolumes)?;
            let bytes = prop.value().as_bytes();
            PodDeserializer::deserialize_from::<Vec<f32>>(bytes)
                .ok()
                .map(|x| x.1)
        }

        if id != ParamType::Props {
            return;
        }

        if let Some(param) = param {
            if let Some(volumes) = pod_get_volume(param) {
                println!("{:?}", volumes);
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = PipewireListener::try_new()?;

    listener.run();

    Ok(())
}
