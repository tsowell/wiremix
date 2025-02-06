use crate::object::ObjectId;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Command {
    NodeMute(ObjectId, bool),
    DeviceMute(ObjectId, i32, i32, bool),
    NodeVolumes(ObjectId, Vec<f32>),
    DeviceVolumes(ObjectId, i32, i32, Vec<f32>),
    NodeCaptureStart(ObjectId, i32, bool),
    NodeCaptureStop(ObjectId),
    MetadataSetProperty(ObjectId, u32, String, Option<String>, Option<String>),
}
