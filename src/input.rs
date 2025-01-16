use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use crossterm::event::EventStream;
use futures::{channel::oneshot, FutureExt, StreamExt};
use futures_timer::Delay;

use crate::message::{InputMessage, Message};

pub fn input_thread_spawn(
    monitor_tx: Arc<mpsc::Sender<Message>>,
) -> oneshot::Sender<()> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    thread::spawn(move || {
        futures::executor::block_on(async move {
            input_loop(shutdown_rx, monitor_tx).await;
        });
    });

    shutdown_tx
}

async fn input_loop(
    shutdown_rx: oneshot::Receiver<()>,
    monitor_tx: Arc<mpsc::Sender<Message>>,
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
                        let _ = monitor_tx.send(Message::Input(InputMessage::Event(event)));
                    }
                    None => break,
                    _ => {},
                }
            }
        }
    }
}
