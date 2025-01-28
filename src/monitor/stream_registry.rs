use std::collections::HashMap;
use std::rc::Rc;

use anyhow::Result;

use nix::sys::eventfd::{EfdFlags, EventFd};

use pipewire::stream::{Stream, StreamListener};

use crate::object::ObjectId;

pub struct StreamRegistry<D> {
    streams: HashMap<ObjectId, Rc<Stream>>,
    listeners: HashMap<ObjectId, Vec<StreamListener<D>>>,
    garbage_streams: Vec<Rc<Stream>>,
    garbage_listeners: Vec<StreamListener<D>>,
    gc_fd: EventFd,
}

impl<D> Drop for StreamRegistry<D> {
    fn drop(&mut self) {
        /* Drop listeners while the stream is still alive. */
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
            garbage_streams: Default::default(),
            garbage_listeners: Default::default(),
            gc_fd,
        })
    }

    pub fn gc_fd(&self) -> &EventFd {
        &self.gc_fd
    }

    pub fn collect_garbage(&mut self) {
        self.garbage_listeners.clear();
        self.garbage_streams.clear();
        let _ = self.gc_fd.read();
    }

    pub fn add_stream(
        &mut self,
        stream_id: ObjectId,
        stream: Rc<Stream>,
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

    pub fn remove(&mut self, stream_id: ObjectId) {
        if let Some(stream) = self.streams.remove(&stream_id) {
            let _ = stream.disconnect();
            self.garbage_streams.push(stream);
            let _ = self.gc_fd.arm();
        }
        if let Some(listeners) = self.listeners.get_mut(&stream_id) {
            if !listeners.is_empty() {
                let _ = self.gc_fd.arm();
            }
            self.garbage_listeners.append(listeners);
        }
    }
}
