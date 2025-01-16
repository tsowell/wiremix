// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use std::sync::{mpsc, Arc};
use std::thread;

use anyhow::Result;
use clap::Parser;

use pwmixer::app;
use pwmixer::input;
use pwmixer::monitor::monitor_pipewire;

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

    thread::spawn({
        let monitor_tx = Arc::clone(&monitor_tx);
        move || {
            let opt = Opt::parse();
            let _ = monitor_pipewire(opt.remote, monitor_tx, !opt.no_capture);
        }
    });

    /* Thread will get cleaned up when shutdown sender is dropped. */
    let _input_shutdown = input::input_thread_spawn(Arc::clone(&monitor_tx));

    let mut terminal = ratatui::init();
    let app_result = app::App::new(monitor_rx).run(&mut terminal);
    ratatui::restore();
    app_result

    /*
    for received in rx {
        println!("{:?}", received);
    }

    Ok(())
    */
}
