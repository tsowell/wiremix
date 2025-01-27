use crate::object::ObjectId;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Command {
    NodeVolumes(ObjectId, Vec<f32>),
    DeviceVolumes(ObjectId, i32, i32, Vec<f32>),
    NodeCapture(ObjectId, i32, bool),
}
