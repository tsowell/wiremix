use std::collections::HashMap;
use std::rc::Rc;

use pipewire::stream::{Stream, StreamListener};

use crate::object::ObjectId;

pub struct StreamRegistry<D> {
    streams: HashMap<ObjectId, Rc<Stream>>,
    listeners: HashMap<ObjectId, Vec<StreamListener<D>>>,
}

impl<D> StreamRegistry<D> {
    pub fn new() -> Self {
        Self {
            streams: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    pub fn add_stream(
        &mut self,
        stream_id: ObjectId,
        stream: Rc<Stream>,
        listener: StreamListener<D>,
    ) {
        self.streams.insert(stream_id, stream);

        let v = self.listeners.entry(stream_id).or_default();
        v.push(listener);
    }

    pub fn remove(&mut self, stream_id: ObjectId) {
        if let Some(stream) = self.streams.remove(&stream_id) {
            let _ = stream.disconnect();
        }
        self.listeners.remove(&stream_id);
    }
}
