//! Atomic f32 operations.

use std::sync::atomic::{AtomicU32, Ordering};

/// An atomic f32 backed by `AtomicU32`.
///
/// All operations use `Ordering::Relaxed`.
#[derive(Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self(AtomicU32::new(value.to_bits()))
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn store(&self, value: f32) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }

    pub fn fetch_update<F>(&self, mut f: F) -> Result<f32, f32>
    where
        F: FnMut(f32) -> Option<f32>,
    {
        self.0
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |bits| {
                f(f32::from_bits(bits)).map(|v| v.to_bits())
            })
            .map(f32::from_bits)
            .map_err(f32::from_bits)
    }
}

impl std::fmt::Debug for AtomicF32 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AtomicF32").field(&self.load()).finish()
    }
}

impl From<f32> for AtomicF32 {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}
