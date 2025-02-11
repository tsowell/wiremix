use std::sync::{mpsc, Arc};

use pipewire::main_loop::WeakMainLoop;

use crate::monitor::{Event, MonitorEvent};

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

    pub fn send(&self, event: MonitorEvent) {
        if self.tx.send(Event::Monitor(event)).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_ready(&self) {
        if self.tx.send(Event::Ready).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_error(&self, error: String) {
        if self.tx.send(Event::Error(error)).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }
}
