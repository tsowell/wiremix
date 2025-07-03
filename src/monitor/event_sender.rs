use std::cell::RefCell;

use pipewire::main_loop::WeakMainLoop;

use crate::monitor::{Event, StateEvent};

/// Trait for handling [`Event`]s.
///
/// Returns `true` if the event was handled successfully, `false` if the
/// monitor should shut down.
pub trait EventHandler: Send + 'static {
    fn handle_event(&mut self, event: Event) -> bool;
}

impl<F> EventHandler for F
where
    F: FnMut(Event) -> bool + Send + 'static,
{
    fn handle_event(&mut self, event: Event) -> bool {
        self(event)
    }
}

pub struct EventSender {
    handler: RefCell<Box<dyn EventHandler>>,
    main_loop_weak: WeakMainLoop,
}

impl EventSender {
    pub fn new<F: EventHandler>(
        handler: F,
        main_loop_weak: WeakMainLoop,
    ) -> Self {
        Self {
            handler: RefCell::new(Box::new(handler)),
            main_loop_weak,
        }
    }

    pub fn send(&self, event: StateEvent) {
        if !self.handler.borrow_mut().handle_event(Event::State(event)) {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_ready(&self) {
        if !self.handler.borrow_mut().handle_event(Event::Ready) {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }

    pub fn send_error(&self, error: String) {
        if !self.handler.borrow_mut().handle_event(Event::Error(error)) {
            if let Some(main_loop) = self.main_loop_weak.upgrade() {
                main_loop.quit();
            }
        }
    }
}
