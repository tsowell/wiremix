use std::mem;
use std::rc::Rc;

use pipewire::{
    core::Core,
    properties::properties,
    stream::{Stream, StreamListener},
};

use libspa::{
    param::audio::{AudioFormat, AudioInfoRaw},
    param::format::{MediaSubtype, MediaType},
    param::{format_utils, ParamType},
    pod::{Object, Pod},
};

use crate::event::MonitorEvent;
use crate::monitor::EventSender;
use crate::object::ObjectId;

#[derive(Default)]
pub struct StreamData {
    format: AudioInfoRaw,
    cursor_move: bool,
}

pub fn capture_node(
    core: &Core,
    sender: &Rc<EventSender>,
    obj_id: ObjectId,
    serial: &str,
    capture_sink: bool,
) -> Option<(Rc<Stream>, StreamListener<StreamData>)> {
    let mut props = properties! {
        *pipewire::keys::TARGET_OBJECT => serial.to_string(),
        *pipewire::keys::STREAM_MONITOR => "true",
        *pipewire::keys::NODE_NAME => "pwmixer-capture",
    };
    if capture_sink {
        props.insert(*pipewire::keys::STREAM_CAPTURE_SINK, "true");
    }

    let data = StreamData {
        format: Default::default(),
        cursor_move: false,
    };

    let stream = Stream::new(core, "pwmixer-capture", props).ok()?;
    let stream = Rc::new(stream);
    let listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(move |_stream, user_data, id, param| {
            // NULL means to clear the format
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
            let _ = user_data.format.parse(param);
        })
        .process({
            let sender_weak = Rc::downgrade(sender);

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
                    let mut peaks = Vec::new();
                    for c in 0..n_channels {
                        let mut max: f32 = 0.0;
                        for n in (c..n_samples).step_by(n_channels as usize) {
                            let start = n as usize * mem::size_of::<f32>();
                            let end = start + mem::size_of::<f32>();
                            let chan = &samples[start..end];
                            let f = f32::from_le_bytes(
                                chan.try_into().unwrap_or([0; 4]),
                            );
                            max = max.max(f.abs());
                        }

                        peaks.push(max);
                    }
                    sender.send(MonitorEvent::NodePeaks(obj_id, peaks));
                    user_data.cursor_move = true;
                }
            }
        })
        .register()
        .ok()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::F32LE);
    let pod_obj = Object {
        type_: pipewire::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> =
        pipewire::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pipewire::spa::pod::Value::Object(pod_obj),
        )
        .ok()?
        .0
        .into_inner();

    let mut params = [Pod::from_bytes(&values)?];

    stream
        .connect(
            libspa::utils::Direction::Input,
            None,
            pipewire::stream::StreamFlags::AUTOCONNECT
                | pipewire::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .ok()?;

    Some((stream, listener))
}
