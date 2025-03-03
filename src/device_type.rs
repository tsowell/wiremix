//! Type representing whether a device is sink or source.

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Sink,
    Source,
}
