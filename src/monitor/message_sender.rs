use std::sync::{mpsc, Arc};

use pipewire::main_loop::WeakMainLoop;

use crate::monitor::MonitorMessage;

pub struct MessageSender {
    tx: Arc<mpsc::Sender<MonitorMessage>>,
    main_loop_weak: WeakMainLoop,
}

impl MessageSender {
    pub fn new(
        tx: Arc<mpsc::Sender<MonitorMessage>>,
        main_loop_weak: WeakMainLoop,
    ) -> Self {
        Self { tx, main_loop_weak }
    }

    pub fn send(&self, message: MonitorMessage) {
        if self.tx.send(message).is_err() {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }
}
