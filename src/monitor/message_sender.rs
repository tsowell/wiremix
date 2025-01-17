use std::sync::{mpsc, Arc};

use pipewire::main_loop::WeakMainLoop;

use crate::monitor::{Message, MonitorMessage};

pub struct MessageSender {
    tx: Arc<mpsc::Sender<Message>>,
    main_loop_weak: WeakMainLoop,
}

impl MessageSender {
    pub fn new(
        tx: Arc<mpsc::Sender<Message>>,
        main_loop_weak: WeakMainLoop,
    ) -> Self {
        Self { tx, main_loop_weak }
    }

    pub fn send(&self, message: MonitorMessage) {
        if self.tx.send(Message::Monitor(message)).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_error(&self, error: String) {
        if self.tx.send(Message::Error(error)).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }
}
