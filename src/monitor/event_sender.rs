use std::sync::{mpsc, Arc};

use pipewire::main_loop::WeakMainLoop;

use crate::event::{Event, MonitorEvent, StateEvent};

pub struct EventSender {
    tx: Arc<mpsc::Sender<Event>>,
    main_loop_weak: WeakMainLoop,
}

impl EventSender {
    pub fn new(
        tx: Arc<mpsc::Sender<Event>>,
        main_loop_weak: WeakMainLoop,
    ) -> Self {
        Self { tx, main_loop_weak }
    }

    pub fn send(&self, event: StateEvent) {
        if self
            .tx
            .send(Event::Monitor(MonitorEvent::State(event)))
            .is_err()
        {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_ready(&self) {
        if self.tx.send(Event::Monitor(MonitorEvent::Ready)).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_error(&self, error: String) {
        if self
            .tx
            .send(Event::Monitor(MonitorEvent::Error(error)))
            .is_err()
        {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }
}
