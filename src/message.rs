#[allow(dead_code)]
#[derive(Debug)]
pub enum MonitorMessage {
    DeviceDescription(u32, String),
    NodeDescription(u32, String),
    NodeName(u32, String),
    NodeNick(u32, String),
    DeviceRouteIndex(u32, i32),
    DeviceRouteDescription(u32, i32, String),
    DeviceProfileIndex(u32, i32),
    DeviceProfileDescription(u32, i32, String),
    NodeVolume(u32, f32),
    NodePeak(u32, f32),
    Removed(u32),
}
