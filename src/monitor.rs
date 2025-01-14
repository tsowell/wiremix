mod device;
mod proxy;
mod stream;

use anyhow::Result;
use pipewire as pw;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;
use std::sync::mpsc;

use pw::{
    core::Core,
    device::Device,
    main_loop::{MainLoop, WeakMainLoop},
    node::Node,
    properties::properties,
    proxy::{Listener, ProxyT},
    registry::{GlobalObject, Registry},
    stream::{Stream, StreamListener, StreamState},
    types::ObjectType,
};

use libspa::param::audio::{AudioFormat, AudioInfoRaw};
use libspa::param::format::{MediaSubtype, MediaType};
use libspa::param::{format_utils, ParamType};
use libspa::pod::{
    deserialize::PodDeserializer, Object, Pod, Value, ValueArray,
};
use libspa::utils::dict::DictRef;

use crate::message::MonitorMessage;
use crate::monitor::device::DeviceStatusTracker;
use crate::monitor::proxy::Proxies;
use crate::monitor::stream::Streams;

fn deserialize(param: Option<&Pod>) -> Option<Object> {
    param
        .and_then(|pod| {
            PodDeserializer::deserialize_any_from(pod.as_bytes()).ok()
        })
        .and_then(|(_, value)| match value {
            Value::Object(obj) => Some(obj),
            _ => None,
        })
}

fn node_props(id: u32, param: Object) -> Option<MonitorMessage> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PROP_channelVolumes {
            if let Value::ValueArray(ValueArray::Float(value)) = prop.value {
                if !value.is_empty() {
                    let mean = value.iter().sum::<f32>() / value.len() as f32;
                    let cubic = mean.cbrt();
                    return Some(MonitorMessage::NodeVolume(id, cubic));
                }
            }
        }
    }

    None
}

fn device_route(id: u32, param: Object) -> Option<MonitorMessage> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorMessage::DeviceRouteIndex(id, value));
            }
        }
    }

    None
}

fn device_enum_route(id: u32, param: Object) -> Option<MonitorMessage> {
    let mut index = None;
    let mut description = None;

    for prop in param.properties {
        match prop.key {
            libspa_sys::SPA_PARAM_ROUTE_index => {
                if let Value::Int(value) = prop.value {
                    index = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_ROUTE_description => {
                if let Value::String(value) = prop.value {
                    description = Some(value);
                }
            }
            _ => (),
        }
    }

    Some(MonitorMessage::DeviceRouteDescription(
        id,
        index?,
        description?,
    ))
}

fn device_profile(id: u32, param: Object) -> Option<MonitorMessage> {
    for prop in param.properties {
        if prop.key == libspa_sys::SPA_PARAM_ROUTE_index {
            if let Value::Int(value) = prop.value {
                return Some(MonitorMessage::DeviceProfileIndex(id, value));
            }
        }
    }

    None
}

fn device_enum_profile(id: u32, param: Object) -> Option<MonitorMessage> {
    let mut index = None;
    let mut description = None;

    for prop in param.properties {
        match prop.key {
            libspa_sys::SPA_PARAM_PROFILE_index => {
                if let Value::Int(value) = prop.value {
                    index = Some(value);
                }
            }
            libspa_sys::SPA_PARAM_PROFILE_description => {
                if let Value::String(value) = prop.value {
                    description = Some(value);
                }
            }
            _ => (),
        }
    }

    Some(MonitorMessage::DeviceProfileDescription(
        id,
        index?,
        description?,
    ))
}

struct MessageSender {
    tx: mpsc::Sender<MonitorMessage>,
    main_loop_weak: WeakMainLoop,
}

impl MessageSender {
    fn new(
        tx: mpsc::Sender<MonitorMessage>,
        main_loop_weak: WeakMainLoop,
    ) -> Self {
        Self { tx, main_loop_weak }
    }

    fn send(&self, message: Option<MonitorMessage>) {
        if let Some(message) = message {
            if self.tx.send(message).is_err() {
                if let Some(main_loop) = self.main_loop_weak.upgrade() {
                    main_loop.quit();
                }
            }
        }
    }
}

#[derive(Default)]
struct StreamData {
    format: AudioInfoRaw,
    cursor_move: bool,
}

type ProxyInfo = (Box<Rc<dyn ProxyT>>, Box<dyn Listener>);
type StreamInfo = (Box<Rc<Stream>>, Box<StreamListener<StreamData>>);

fn monitor_node(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<MessageSender>,
) -> Option<ProxyInfo> {
    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Sink" => (),
        "Audio/Source" => (),
        "Stream/Output/Audio" => (),
        _ => return None,
    }

    let node: Node = registry.bind(obj).ok()?;
    let node = Rc::new(node);
    let proxy_id = node.upcast_ref().id();

    if let Some(node_description) = props.get("node.description") {
        let message = MonitorMessage::NodeDescription(
            proxy_id,
            String::from(node_description),
        );
        sender.send(Some(message));
    }

    let listener = node
        .add_listener_local()
        .param({
            let sender_weak = Rc::downgrade(&sender);
            move |_seq, id, _index, _next, param| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                if let Some(param) = deserialize(param) {
                    sender.send(match id {
                        ParamType::Props => node_props(proxy_id, param),
                        _ => None,
                    });
                }
            }
        })
        .register();
    node.subscribe_params(&[ParamType::Props]);

    Some((Box::new(node), Box::new(listener)))
}

fn capture_node(
    core: &Core,
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<MessageSender>,
    proxy_id: u32,
) -> Option<StreamInfo> {
    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Sink" => (),
        "Audio/Source" => (),
        "Stream/Output/Audio" => (),
        _ => return None,
    }

    let props = properties! {
        *pw::keys::TARGET_OBJECT => obj.id.to_string()
    };

    let data = StreamData {
        format: Default::default(),
        cursor_move: false,
    };

    let stream = Stream::new(&core, "pwmixer-capture", props).ok()?;
    let stream = Rc::new(stream);
    let listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(move |_stream, user_data, id, param| {
            // NULL means ot clear the format
            let Some(param) = param else {
                return;
            };
            if id != ParamType::Format.as_raw() {
                return;
            }

            let (media_type, media_subtype) =
                match format_utils::parse_format(param) {
                    Ok(v) => v,
                    Err(_) => return,
                };

            // only accept raw audio
            if media_type != MediaType::Audio
                || media_subtype != MediaSubtype::Raw
            {
                return;
            }

            // call a helper function to parse the format for us.
            if !user_data.format.parse(param).is_ok() {
                return;
            }
        })
        .process({
            let sender_weak = Rc::downgrade(&sender);

            move |stream, user_data| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }

                let data = &mut datas[0];
                let n_channels = user_data.format.channels();
                let n_samples =
                    data.chunk().size() / (mem::size_of::<f32>() as u32);

                if let Some(samples) = data.data() {
                    let mut sum = 0.0;
                    for c in 0..n_channels {
                        let mut max: f32 = 0.0;
                        for n in (c..n_samples).step_by(n_channels as usize) {
                            let start = n as usize * mem::size_of::<f32>();
                            let end = start + mem::size_of::<f32>();
                            let chan = &samples[start..end];
                            let f =
                                f32::from_le_bytes(chan.try_into().unwrap());
                            max = max.max(f.abs());
                        }

                        sum += max;
                    }
                    let average = sum as f32 / n_channels as f32;
                    sender.send(Some(MonitorMessage::NodePeak(
                        proxy_id, average,
                    )));
                    user_data.cursor_move = true;
                }
            }
        })
        .register()
        .ok()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::F32LE);
    let pod_obj = Object {
        type_: pw::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(pod_obj),
    )
    .ok()?
    .0
    .into_inner();

    let mut params = [Pod::from_bytes(&values)?];

    stream
        .connect(
            libspa::utils::Direction::Input,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .ok()?;

    println!("{}", proxy_id);
    Some((Box::new(stream), Box::new(listener)))
}

fn monitor_device(
    registry: &Registry,
    obj: &GlobalObject<&DictRef>,
    sender: &Rc<MessageSender>,
    statuses: &Rc<RefCell<DeviceStatusTracker>>,
) -> Option<ProxyInfo> {
    let props = obj.props?;
    let media_class = props.get("media.class")?;
    match media_class {
        "Audio/Device" => (),
        _ => return None,
    }

    let device: Device = registry.bind(obj).ok()?;
    let device = Rc::new(device);
    let proxy_id = device.upcast_ref().id();

    if let Some(device_description) = props.get("device.description") {
        let message = MonitorMessage::DeviceDescription(
            proxy_id,
            String::from(device_description),
        );
        sender.send(Some(message));
    }

    let params = [
        ParamType::Route,
        ParamType::EnumRoute,
        ParamType::Profile,
        ParamType::EnumProfile,
    ];

    let listener = device
        .add_listener_local()
        .param({
            let sender_weak = Rc::downgrade(&sender);
            let statuses_weak = Rc::downgrade(&statuses);
            move |_seq, id, _index, _next, param| {
                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                let Some(statuses) = statuses_weak.upgrade() else {
                    return;
                };
                if let Some(param) = deserialize(param) {
                    sender.send(match id {
                        ParamType::Route => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_route(proxy_id, param)
                        }
                        ParamType::EnumRoute => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_enum_route(proxy_id, param)
                        }
                        ParamType::Profile => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_profile(proxy_id, param)
                        }
                        ParamType::EnumProfile => {
                            statuses.borrow_mut().set(proxy_id, id);
                            device_enum_profile(proxy_id, param)
                        }
                        _ => None,
                    });
                }
            }
        })
        .info({
            let device_weak = Rc::downgrade(&device);
            let statuses_weak = Rc::downgrade(&statuses);
            move |_info| {
                let Some(device) = device_weak.upgrade() else {
                    return;
                };
                let Some(statuses) = statuses_weak.upgrade() else {
                    return;
                };
                let statuses = statuses.borrow();
                let Some(status) = statuses.get(proxy_id) else {
                    return;
                };
                for param in params.into_iter() {
                    if !status.get(param) {
                        device.enum_params(0, Some(param), 0, u32::MAX);
                    }
                }
            }
        })
        .register();

    device.subscribe_params(&params);

    Some((Box::new(device), Box::new(listener)))
}

pub fn monitor_pipewire(
    remote: Option<String>,
    tx: mpsc::Sender<MonitorMessage>,
) -> Result<()> {
    pw::init();

    let main_loop = MainLoop::new(None)?;

    let context = pw::context::Context::new(&main_loop)?;
    let props = remote.map(|remote| {
        properties! {
            *pw::keys::REMOTE_NAME => remote
        }
    });
    let core = context.connect(props)?;

    let registry = Rc::new(core.get_registry()?);
    let registry_weak = Rc::downgrade(&registry);

    // Proxies and their listeners need to stay alive so store them here
    let proxies = Rc::new(RefCell::new(Proxies::new()));
    let streams = Rc::new(RefCell::new(Streams::new()));

    let statuses = Rc::new(RefCell::new(DeviceStatusTracker::new()));

    let sender = Rc::new(MessageSender::new(tx, main_loop.downgrade()));
    let _registry_listener = registry
        .add_listener_local()
        .global(move |obj| {
            let Some(registry) = registry_weak.upgrade() else {
                return;
            };
            let (p, s): (Option<ProxyInfo>, Option<StreamInfo>) = match obj
                .type_
            {
                ObjectType::Node => {
                    let p = monitor_node(&registry, obj, &sender);
                    if let Some((ref proxy, _)) = p {
                        let id = proxy.upcast_ref().id();
                        (p, capture_node(&core, &registry, obj, &sender, id))
                    } else {
                        (p, None)
                    }
                }
                ObjectType::Device => {
                    (monitor_device(&registry, obj, &sender, &statuses), None)
                }
                _ => (None, None),
            };

            if let Some((proxy_spe, listener_spe)) = p {
                let proxy = proxy_spe.upcast_ref();
                let proxy_id = proxy.id();
                // Use a weak ref to prevent references cycle between Proxy and proxies:
                // - ref on proxies in the closure, bound to the Proxy lifetime
                // - proxies owning a ref on Proxy as well
                let proxies_weak = Rc::downgrade(&proxies);

                let sender_weak = Rc::downgrade(&sender);
                let listener = proxy
                    .add_listener_local()
                    .removed(move || {
                        let Some(sender) = sender_weak.upgrade() else {
                            return;
                        };
                        if let Some(proxies) = proxies_weak.upgrade() {
                            proxies.borrow_mut().remove(proxy_id);
                            let message = MonitorMessage::Removed(proxy_id);
                            sender.send(Some(message));
                        }
                    })
                    .register();

                let mut proxies = proxies.borrow_mut();
                proxies.add_proxy_t(proxy_spe, listener_spe);
                proxies.add_proxy_listener(proxy_id, listener);

                if let Some((stream_spe, listener_spe)) = s {
                    let streams_weak = Rc::downgrade(&streams);
                    let stream_spe_weak = Rc::downgrade(&stream_spe);
                    let listener = stream_spe
                        .add_local_listener()
                        .state_changed(
                            move |stream, user_data, old_state, new_state| {
                                let Some(streams) = streams_weak.upgrade()
                                else {
                                    return;
                                };
                                let Some(stream_spe) =
                                    stream_spe_weak.upgrade()
                                else {
                                    return;
                                };
                                match new_state {
                                    StreamState::Error(_)
                                    | StreamState::Unconnected => {
                                        stream_spe.disconnect();
                                        streams.borrow_mut().remove(proxy_id);
                                    }
                                    _ => (),
                                }
                            },
                        )
                        .register();

                    let Ok(listener) = listener else {
                        return;
                    };

                    let mut streams = streams.borrow_mut();
                    streams.add_stream(proxy_id, stream_spe, listener_spe);
                    streams.add_stream_listener(proxy_id, listener);
                }
            }
        })
        .register();

    main_loop.run();

    unsafe {
        pw::deinit();
    }

    Ok(())
}
