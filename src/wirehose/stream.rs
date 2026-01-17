use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

use pulp::{Arch, Simd, WithSimd};

use crate::atomic_f32::AtomicF32;
use crate::wirehose::event_sender::EventSender;
use crate::wirehose::{ObjectId, StateEvent};

/// Trait for processing peaks in order to implement effects like ballistics.
pub trait PeakProcessor: Send + Sync {
    fn process_peak(
        &self,
        current_peak: f32,
        previous_peak: f32,
        sample_count: u32,
        sample_rate: u32,
    ) -> f32;
}

impl<F> PeakProcessor for F
where
    F: Fn(f32, f32, u32, u32) -> f32 + Send + Sync,
{
    fn process_peak(
        &self,
        current_peak: f32,
        previous_peak: f32,
        sample_count: u32,
        sample_rate: u32,
    ) -> f32 {
        self(current_peak, previous_peak, sample_count, sample_rate)
    }
}

#[derive(Default)]
pub struct StreamData {
    format: AudioInfoRaw,
    peaks: Arc<[AtomicF32]>,
    peaks_dirty: Arc<AtomicBool>,
    peak_processor: Option<Arc<dyn PeakProcessor>>,
    buffers_seen: u8,
}

fn find_peak(samples: &[f32]) -> f32 {
    struct Max<'a>(&'a [f32]);
    impl WithSimd for Max<'_> {
        type Output = f32;

        #[inline(always)]
        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            let v = self.0;

            let (head, tail) = S::as_simd_f32s(v);

            let mut head_max = simd.splat_f32s(0.0);
            for x in head {
                head_max = simd.max_f32s(head_max, simd.abs_f32s(*x));
            }
            let head_max = head_max;

            let mut tail_max = simd.reduce_max_f32s(head_max);
            for x in tail {
                tail_max = tail_max.max(x.abs());
            }

            tail_max
        }
    }

    Arch::new().dispatch(Max(samples))
}

pub fn capture_node(
    core: &Core,
    sender: &Rc<EventSender>,
    object_id: ObjectId,
    serial: &str,
    capture_sink: bool,
    peaks_dirty: Arc<AtomicBool>,
    peak_processor: Option<Arc<dyn PeakProcessor>>,
) -> Option<(Rc<Stream>, StreamListener<StreamData>)> {
    let mut props = properties! {
        *pipewire::keys::TARGET_OBJECT => String::from(serial),
        *pipewire::keys::STREAM_MONITOR => "true",
        *pipewire::keys::NODE_NAME => "wiremix-capture",
    };
    if capture_sink {
        props.insert(*pipewire::keys::STREAM_CAPTURE_SINK, "true");
    }

    let data = StreamData {
        format: Default::default(),
        peaks: Default::default(),
        peaks_dirty,
        peak_processor,
        buffers_seen: 0,
    };

    let stream = Stream::new(core, "wiremix-capture", props).ok()?;
    let stream = Rc::new(stream);
    let listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed({
            let sender_weak = Rc::downgrade(sender);

            move |_stream, user_data, id, param| {
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
                user_data.peaks = (0..user_data.format.channels())
                    .map(|_| AtomicF32::new(0.0))
                    .collect::<Arc<[_]>>();

                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };
                sender.send(StateEvent::NodeStreamStarted {
                    object_id,
                    rate: user_data.format.rate(),
                    peaks: Arc::clone(&user_data.peaks),
                });
            }
        })
        .process({
            let sender_weak = Rc::downgrade(sender);

            move |stream, user_data| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };

                user_data.buffers_seen =
                    user_data.buffers_seen.saturating_add(1);

                let Some(sender) = sender_weak.upgrade() else {
                    return;
                };

                // For some reason the first chunk's samples are all 0s.
                // Also, the second bit of the flags is set despite not being
                // defined on ChunkFlags. It would be nice to know what's going
                // on there, but until then, skip the first buffer after
                // starting capture.
                if user_data.buffers_seen == 1 {
                    return;
                }

                let datas = buffer.datas_mut();
                if datas.is_empty() {
                    return;
                }

                let n_channels = user_data.format.channels() as usize;
                let mut n_samples = 0u32;

                for c in 0..n_channels {
                    let Some(data) = datas.get_mut(c) else {
                        user_data.peaks[c].store(0.0);
                        continue;
                    };

                    let chunk_size = data.chunk().size() as usize;

                    let Some(samples) = data.data() else {
                        user_data.peaks[c].store(0.0);
                        continue;
                    };

                    let samples: &[f32] =
                        bytemuck::cast_slice(&samples[..chunk_size]);
                    if c == 0 {
                        n_samples = samples.len() as u32;
                    }

                    let new_peak = find_peak(samples);

                    if let Some(peak_processor) = &user_data.peak_processor {
                        let _ = user_data.peaks[c].fetch_update(|current| {
                            // Initialize if this is the first peak.
                            let current = if user_data.buffers_seen == 2 {
                                new_peak
                            } else {
                                current
                            };
                            Some(peak_processor.process_peak(
                                new_peak,
                                current,
                                n_samples,
                                user_data.format.rate(),
                            ))
                        });
                    } else {
                        user_data.peaks[c].store(new_peak);
                    }
                }

                if !user_data.peaks_dirty.swap(true, Ordering::Relaxed) {
                    sender.send(StateEvent::NodePeaksDirty { object_id });
                }
            }
        })
        .register()
        .ok()?;

    let mut audio_info = AudioInfoRaw::new();
    audio_info.set_format(AudioFormat::F32P);
    let pod_object = Object {
        type_: pipewire::spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> =
        pipewire::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pipewire::spa::pod::Value::Object(pod_object),
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
