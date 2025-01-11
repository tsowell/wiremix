use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use env_logger;
use log::{info, warn};

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
use libspa::pod::{deserialize::PodDeserializer, Pod, Value, ValueArray};
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
                    info!("{:?}", global);
                    let node_id = global.id;
                    let listener = node
                        .add_listener_local()
                        .param(move |_, id, _, _, param| {
                            Self::node_param(node_id, id, param)
                        })
                        .register();
                    node.subscribe_params(&[ParamType::Props]);
                    nodes.borrow_mut().insert(node_id, (node, listener));
                }
            })
            .global_remove(move |id| {
                if nodes_remove.borrow_mut().remove(&id).is_some() {
                    info!("Removed: {}", id);
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

    fn node_param_prop_channel_volumes(node_id: u32, value: &Vec<f32>) {
        if !value.is_empty() {
            let mean = value.iter().sum::<f32>() / value.len() as f32;
            let cubic = mean.cbrt();
            println!("{} {:?}", node_id, cubic);
        }
    }

    fn node_param(node_id: u32, id: ParamType, param: Option<&Pod>) {
        if id != ParamType::Props {
            warn!("Unhandled ParamType {:?}", id);
            return;
        }

        let Some(param) = param else {
            return;
        };

        let Ok((_, value)) =
            PodDeserializer::deserialize_any_from(param.as_bytes())
        else {
            return;
        };

        let Value::Object(obj) = value else {
            warn!("Unhandled param value {:?}", value);
            return;
        };

        for prop in obj.properties {
            match prop.key {
                libspa_sys::SPA_PROP_channelVolumes => {
                    if let Value::ValueArray(ValueArray::Float(value)) =
                        &prop.value
                    {
                        Self::node_param_prop_channel_volumes(node_id, value);
                    }
                }
                _ => (),
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let listener = PipewireListener::try_new()?;

    listener.run();

    Ok(())
}
