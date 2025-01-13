use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use log::{info, warn};

use pipewire::{
    context::Context,
    core::Core,
    main_loop::MainLoop,
    node::Node,
    device::Device,
    proxy::{Listener as ProxyListener, ProxyT},
    registry::{GlobalObject, Listener, Registry},
    types::ObjectType,
};

use libspa::param::ParamType;
use libspa::pod::{deserialize::PodDeserializer, Object, Pod, Value, ValueArray};
use libspa::utils::dict::DictRef;

const DEVICE_MEDIA_CLASSES: &[&str] = &[
    "Audio/Device",
];

const NODE_MEDIA_CLASSES: &[&str] = &[
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
        let objects = Rc::new(RefCell::new(HashMap::<
            u32,
            (Box<dyn ProxyT>, Box<dyn ProxyListener>),
        >::new()));

        let objects_bind = Rc::clone(&objects);
        let objects_remove = Rc::clone(&objects);
        let registry_bind = Rc::clone(&registry);

        let listener = registry
            .borrow()
            .add_listener_local()
            .global(move |global| {
                if let Some(object) =
                    Self::bind(&registry_bind.borrow(), global)
                {
                    info!("{:?}", global);
                    objects_bind.borrow_mut().insert(global.id, object);
                }
            })
            .global_remove(move |id| {
                if objects_remove.borrow_mut().remove(&id).is_some() {
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

    pub fn bind(
        registry: &Registry,
        global: &GlobalObject<&DictRef>,
    ) -> Option<(Box<dyn ProxyT>, Box<dyn ProxyListener>)> {
        if global.type_ == ObjectType::Node {
            let props = global.props?;
            let media_class = props.get("media.class")?;
            if !NODE_MEDIA_CLASSES.contains(&media_class) {
                return None;
            }

            println!("Node {}", media_class);

            let node = registry.bind::<Node, &DictRef>(global).ok()?;
            let global_id = global.id;
            let listener = node
                .add_listener_local()
                .param(move |_, id, _, _, param| {
                    Self::param(global_id, id, param)
                })
                .register();
            node.subscribe_params(&[ParamType::Props]);
            Some((Box::new(node), Box::new(listener)))
        } else if global.type_ == ObjectType::Device {
            let props = global.props?;
            let media_class = props.get("media.class")?;
            if !DEVICE_MEDIA_CLASSES.contains(&media_class) {
                return None;
            }

            println!("Device {}", media_class);

            let node = registry.bind::<Device, &DictRef>(global).ok()?;
            let global_id = global.id;
            let listener = node
                .add_listener_local()
                .param(move |_, id, _, _, param| {
                    Self::param(global_id, id, param)
                })
                .register();
            node.subscribe_params(&[
                ParamType::Route,
                ParamType::EnumRoute,
                ParamType::Profile,
                ParamType::EnumProfile,
            ]);
            Some((Box::new(node), Box::new(listener)))
        } else {
            None
        }
    }

    pub fn run(&self) {
        self.mainloop.run()
    }

    fn param_props(global_id: u32, object: Object) {
        for prop in object.properties {
            if prop.key == libspa_sys::SPA_PROP_channelVolumes {
                Self::prop_channel_volumes(global_id, &prop.value);
            }
        }
    }

    fn prop_channel_volumes(global_id: u32, value: &Value) {
        let Value::ValueArray(ValueArray::Float(value)) = value else {
            return;
        };

        if !value.is_empty() {
            let mean = value.iter().sum::<f32>() / value.len() as f32;
            let cubic = mean.cbrt();
            println!("{} {:?}", global_id, cubic);
        }
    }

    fn param_route(global_id: u32, object: Object) {
        for prop in object.properties {
            if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
                Self::param_route_index(global_id, &prop.value);
            }
        }
    }

    fn param_route_index(global_id: u32, value: &Value) {
        let Value::Int(value) = value else { return };

        println!("{} route index {:?}", global_id, value);
    }

    fn param_enum_route(global_id: u32, object: Object) {
        let mut index = None;
        let mut description = None;

        for prop in object.properties {
            match prop.key {
                libspa_sys::SPA_PARAM_ROUTE_index => {
                    if let Value::Int(value) = prop.value {
                        index = Some(value);
                    }
                },
                libspa_sys::SPA_PARAM_ROUTE_description => {
                    if let Value::String(value) = prop.value {
                        description = Some(value);
                    }
                },
                _ => (),
            }
        }

        let Some(index) = index else { return; };
        let Some(description) = description else { return; };

        println!("{} route {}: {}", global_id, index, description);
    }

    fn param_profile(global_id: u32, object: Object) {
        for prop in object.properties {
            if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
                Self::param_profile_index(global_id, &prop.value);
            }
        }
    }

    fn param_profile_index(global_id: u32, value: &Value) {
        let Value::Int(value) = value else { return };

        println!("{} profile index {:?}", global_id, value);
    }

    fn param_enum_profile(global_id: u32, object: Object) {
        let mut index = None;
        let mut description = None;

        for prop in object.properties {
            match prop.key {
                libspa_sys::SPA_PARAM_PROFILE_index => {
                    if let Value::Int(value) = prop.value {
                        index = Some(value);
                    }
                },
                libspa_sys::SPA_PARAM_PROFILE_description => {
                    if let Value::String(value) = prop.value {
                        description = Some(value);
                    }
                },
                _ => (),
            }
        }

        let Some(index) = index else { return; };
        let Some(description) = description else { return; };

        println!("{} profile {}: {}", global_id, index, description);
    }

    fn param(global_id: u32, id: ParamType, param: Option<&Pod>) {
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

        match id {
            ParamType::Props => Self::param_props(global_id, obj),
            ParamType::Route => Self::param_route(global_id, obj),
            ParamType::EnumRoute => Self::param_enum_route(global_id, obj),
            ParamType::Profile => Self::param_profile(global_id, obj),
            ParamType::EnumProfile => Self::param_enum_profile(global_id, obj),
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let listener = PipewireListener::try_new()?;

    listener.run();

    Ok(())
}
