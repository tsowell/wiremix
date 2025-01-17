use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use crossterm::event::EventStream;
use futures::{channel::oneshot, FutureExt, StreamExt};
use futures_timer::Delay;

use crate::message::Message;

pub fn spawn(tx: Arc<mpsc::Sender<Message>>) -> InputHandle {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let handle = thread::spawn(move || {
        futures::executor::block_on(async move {
            input_loop(shutdown_rx, tx).await;
        });
    });

    InputHandle {
        tx: Some(shutdown_tx),
        handle: Some(handle),
    }
}

pub struct InputHandle {
    tx: Option<oneshot::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for InputHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

async fn input_loop(
    shutdown_rx: oneshot::Receiver<()>,
    tx: Arc<mpsc::Sender<Message>>,
) {
    let mut reader = EventStream::new();
    let mut shutdown = shutdown_rx.fuse();

    loop {
        let mut delay = Delay::new(Duration::from_millis(1_000)).fuse();
        let mut event = reader.next().fuse();

        futures::select! {
            _ = shutdown => break,
            _ = delay => { },
            maybe_event = event => {
                match maybe_event {
                    Some(Ok(event)) => {
                        let _ = tx.send(Message::from(event));
                    }
                    None => break,
                    _ => {},
                }
            }
        }
    }
}
