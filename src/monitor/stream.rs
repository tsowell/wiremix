use std::collections::HashMap;
use std::rc::Rc;

use pipewire::stream::{Stream, StreamListener};

pub struct Streams<D> {
    streams: HashMap<u32, Rc<Stream>>,
    listeners: HashMap<u32, Vec<StreamListener<D>>>,
}

impl<D> Streams<D> {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    pub fn add_stream(
        &mut self,
        stream_id: u32,
        stream: Rc<Stream>,
        listener: StreamListener<D>,
    ) {
        self.streams.insert(stream_id, stream);

        let v = self.listeners.entry(stream_id).or_default();
        v.push(listener);
    }

    pub fn add_stream_listener(
        &mut self,
        stream_id: u32,
        listener: StreamListener<D>,
    ) {
        let v = self.listeners.entry(stream_id).or_default();
        v.push(listener);
    }

    pub fn remove(&mut self, stream_id: u32) {
        self.streams.remove(&stream_id);
        self.listeners.remove(&stream_id);
    }
}
