use pipewire::{
    main_loop::MainLoop,
    context::Context,
    registry::GlobalObject,
    types::ObjectType
};
use libspa::utils::dict::DictRef;

const MEDIA_CLASSES: &[&str] = &[
    "Audio/Device",
    "Audio/Sink",
    "Audio/Source",
    "Stream/Output/Audio",
];

fn global(global: &GlobalObject<&DictRef>) {
    match global.type_ {
        ObjectType::Node => {
            if let Some(props) = &global.props {
                if let Some(media_class) = props.get("media.class") {
                    if MEDIA_CLASSES.contains(&media_class) {
                        println!("{}: {:?}", media_class, global);
                    }
                }
            }
        },
        _ => (),
    }
}

fn global_remove(global_remove: u32) {
    println!("Remove: {}", global_remove);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let _listener = registry
        .add_listener_local()
        .global(global)
        .global_remove(global_remove)
        .register();

    mainloop.run();

    Ok(())
}
