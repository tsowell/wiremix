//! PipeWire controls which can be executed by wirehose.

use std::sync::{atomic::AtomicBool, Arc};

use crate::wirehose::{state::PeakProcessor, ObjectId};

pub enum Command {
    NodeMute(ObjectId, bool),
    DeviceMute(ObjectId, i32, i32, bool),
    NodeVolumes(ObjectId, Vec<f32>),
    DeviceVolumes(ObjectId, i32, i32, Vec<f32>),
    DeviceSetRoute(ObjectId, i32, i32),
    DeviceSetProfile(ObjectId, i32),
    NodeCaptureStart(
        ObjectId,
        u64,
        bool,
        Arc<AtomicBool>,
        Option<Arc<dyn PeakProcessor>>,
    ),
    NodeCaptureStop(ObjectId),
    MetadataSetProperty(ObjectId, u32, String, Option<String>, Option<String>),
}

/// Trait for sending commands to control PipeWire. The trait exists to
/// facilitate mocking.
pub trait CommandSender {
    fn node_capture_start(
        &self,
        obj_id: ObjectId,
        object_serial: u64,
        capture_sink: bool,
        peaks_dirty: Arc<AtomicBool>,
        peak_processor: Option<Arc<dyn PeakProcessor>>,
    );
    fn node_capture_stop(&self, obj_id: ObjectId);
    fn node_mute(&self, obj_id: ObjectId, mute: bool);
    fn node_volumes(&self, obj_id: ObjectId, volumes: Vec<f32>);
    fn device_mute(
        &self,
        obj_id: ObjectId,
        route_index: i32,
        route_device: i32,
        mute: bool,
    );
    fn device_set_profile(&self, obj_id: ObjectId, profile_index: i32);
    fn device_set_route(
        &self,
        obj_id: ObjectId,
        route_index: i32,
        route_device: i32,
    );
    fn device_volumes(
        &self,
        obj_id: ObjectId,
        route_index: i32,
        route_device: i32,
        volumes: Vec<f32>,
    );
    fn metadata_set_property(
        &self,
        obj_id: ObjectId,
        subject: u32,
        key: String,
        type_: Option<String>,
        value: Option<String>,
    );
}
