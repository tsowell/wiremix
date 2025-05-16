//! Type representing whether a device is sink or source.

#[derive(Debug, Clone, Copy)]
pub enum DeviceKind {
    Sink,
    Source,
}
