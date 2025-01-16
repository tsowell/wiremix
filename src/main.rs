// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::sync::{mpsc, Arc};

use anyhow::Result;
use clap::Parser;

use pwmixer::app;
use pwmixer::input;
use pwmixer::monitor;

#[derive(Parser)]
#[clap(name = "pwmixer", about = "PipeWire mixer")]
struct Opt {
    #[clap(short, long, help = "The name of the remote to connect to")]
    remote: Option<String>,

    #[clap(short, long, help = "Disable audio capture for level monitoring)]")]
    no_capture: bool,
}

fn main() -> Result<()> {
    let (monitor_tx, monitor_rx) = mpsc::channel();
    let monitor_tx = Arc::new(monitor_tx);

    let opt = Opt::parse();
    let (monitor_thread, monitor_shutdown) =
        monitor::spawn(opt.remote, Arc::clone(&monitor_tx), !opt.no_capture)?;

    // Thread will get cleaned up when shutdown sender is dropped.
    let (input_thread, input_shutdown) = input::spawn(Arc::clone(&monitor_tx));

    let mut terminal = ratatui::init();
    let app_result =
        app::App::new(monitor_rx).run(&mut terminal);
    ratatui::restore();

    input_shutdown.trigger();
    monitor_shutdown.trigger();

    input_thread.join().unwrap();
    monitor_thread.join().unwrap();

    app_result

    /*
    for received in monitor_rx {
        if let pwmixer::message::Message::Monitor(message) = received {
            println!("{:?}", message);
        }
    }

    Ok(())
    */
}
