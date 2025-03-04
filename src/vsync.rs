//! Setup and teardown of vsync timer.
//!
//! [`spawn()`] starts the vsync thead.

use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use futures::{channel::oneshot, FutureExt};
use futures_timer::Delay;

use crate::event::Event;

/// Spawns a thread to generate Vsync events.
///
/// [`Event`](`crate::event::Event`)s are sent to tx.
///
/// Returns a [`VsyncHandle`] to automatically clean up the thread.
pub fn spawn(tx: Arc<mpsc::Sender<Event>>, fps: f32) -> VsyncHandle {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let handle = thread::spawn(move || {
        futures::executor::block_on(async move {
            vsync_loop(shutdown_rx, tx, fps).await;
        });
    });

    VsyncHandle {
        tx: Some(shutdown_tx),
        handle: Some(handle),
    }
}

/// Handle for the vsync thread.
///
/// On cleanup, the thread will be notified to quit and will be joined.
pub struct VsyncHandle {
    tx: Option<oneshot::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for VsyncHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

async fn vsync_loop(
    shutdown_rx: oneshot::Receiver<()>,
    tx: Arc<mpsc::Sender<Event>>,
    fps: f32,
) {
    let mut shutdown = shutdown_rx.fuse();

    let frame_duration = Duration::from_secs_f32(1.0 / fps);

    loop {
        let start = Instant::now();

        let _ = tx.send(Event::Vsync);

        let elapsed = start.elapsed();
        let delay_duration = if elapsed < frame_duration {
            frame_duration - elapsed
        } else {
            Duration::from_millis(1)
        };

        let mut delay = Delay::new(delay_duration).fuse();

        futures::select! {
            _ = shutdown => break,
            _ = delay => { },
        }
    }
}
