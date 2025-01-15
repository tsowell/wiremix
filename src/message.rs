use libspa::utils::dict::DictRef;
use pipewire::registry::GlobalObject;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(u32);

impl From<&GlobalObject<&DictRef>> for ObjectId {
    fn from(obj: &GlobalObject<&DictRef>) -> Self {
        ObjectId(obj.id)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum MonitorMessage {
    DeviceDescription(ObjectId, String),
    DeviceName(ObjectId, String),
    DeviceNick(ObjectId, String),
    NodeDescription(ObjectId, String),
    NodeName(ObjectId, String),
    NodeNick(ObjectId, String),
    NodeMediaName(ObjectId, String),
    DeviceRouteIndex(ObjectId, i32),
    DeviceRouteDescription(ObjectId, i32, String),
    DeviceProfileIndex(ObjectId, i32),
    DeviceProfileDescription(ObjectId, i32, String),
    NodeVolume(ObjectId, f32),
    NodePeak(ObjectId, f32),
    Removed(ObjectId),
}
