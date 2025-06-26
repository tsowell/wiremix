//! PipeWire controls which can be executed by the monitor module.

use crate::object::ObjectId;

#[derive(Debug)]
pub enum Command {
    NodeMute(ObjectId, bool),
    DeviceMute(ObjectId, i32, i32, bool),
    NodeVolumes(ObjectId, Vec<f32>),
    DeviceVolumes(ObjectId, i32, i32, Vec<f32>),
    DeviceSetRoute(ObjectId, i32, i32),
    DeviceSetProfile(ObjectId, i32),
    NodeCaptureStart(ObjectId, u64, bool),
    NodeCaptureStop(ObjectId),
    MetadataSetProperty(ObjectId, u32, String, Option<String>, Option<String>),
}
