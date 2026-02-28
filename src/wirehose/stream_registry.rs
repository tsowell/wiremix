use std::collections::{HashMap, HashSet};

use anyhow::Result;

use nix::sys::eventfd::{EfdFlags, EventFd};

use pipewire::stream::{StreamListener, StreamRc};

use crate::wirehose::ObjectId;

/// Storage for keeping streams and their listeners alive
pub struct StreamRegistry<D> {
    /// Storage for keeping streams
    streams: HashMap<ObjectId, StreamRc>,
    /// Storage for keeping listeners alive
    listeners: HashMap<ObjectId, Vec<StreamListener<D>>>,
    /// Streams pending deletion
    garbage_streams: Vec<StreamRc>,
    /// Listeners pending deletion
    garbage_listeners: Vec<StreamListener<D>>,
    /// Track garbage node IDs so [`Self::collect_garbage()`] can report on who
    /// was collected.
    garbage_ids: HashSet<ObjectId>,
    /// EventFd for signalling to [`wirehose`](`crate::wirehose`) that objects
    /// are pending deletion and that [`Self::collect_garbage()`] needs to be
    /// called
    pub gc_fd: EventFd,
}

impl<D> Drop for StreamRegistry<D> {
    fn drop(&mut self) {
        // Drop listeners while the stream is still alive.
        self.garbage_listeners.clear();
        self.listeners.clear();
    }
}

impl<D> StreamRegistry<D> {
    pub fn try_new() -> Result<Self> {
        let gc_fd = EventFd::from_value_and_flags(0, EfdFlags::EFD_NONBLOCK)?;
        Ok(Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
            garbage_streams: Vec::new(),
            garbage_listeners: Vec::new(),
            garbage_ids: HashSet::new(),
            gc_fd,
        })
    }

    /// Clean up streams and listeners pending deletion. It is unsafe to call
    /// this from within the PipeWire main loop!
    ///
    /// Returns the IDs of the streams deleted.
    pub fn collect_garbage(&mut self) -> Vec<ObjectId> {
        self.garbage_listeners.clear();
        self.garbage_streams.clear();
        let _ = self.gc_fd.read();
        self.garbage_ids.drain().collect()
    }

    /// Register a stream and its listener, evicting any with the same ID.
    pub fn add_stream(
        &mut self,
        stream_id: ObjectId,
        stream: StreamRc,
        listener: StreamListener<D>,
    ) {
        if let Some(old) = self.streams.insert(stream_id, stream) {
            self.garbage_streams.push(old);
            if let Some(listeners) = self.listeners.get_mut(&stream_id) {
                self.garbage_listeners.append(listeners);
            }
            let _ = self.gc_fd.arm();
        }

        let v = self.listeners.entry(stream_id).or_default();
        v.push(listener);
    }

    /// Remove a stream, deferring deletion until [`Self::collect_garbage()`]
    /// is called.
    pub fn remove(&mut self, stream_id: ObjectId) {
        if let Some(stream) = self.streams.remove(&stream_id) {
            let _ = stream.disconnect();
            self.garbage_streams.push(stream);
            self.garbage_ids.insert(stream_id);
            let _ = self.gc_fd.arm();
        }
        if let Some(listeners) = self.listeners.get_mut(&stream_id) {
            if !listeners.is_empty() {
                let _ = self.gc_fd.arm();
                self.garbage_ids.insert(stream_id);
            }
            self.garbage_listeners.append(listeners);
        }
    }
}
