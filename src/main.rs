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
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(tx);

    let opt = Opt::parse();
    let _monitor_handle =
        monitor::spawn(opt.remote, Arc::clone(&tx), !opt.no_capture)?;
    let _input_handle = input::spawn(Arc::clone(&tx));

    let mut terminal = ratatui::init();
    let app_result = app::App::new(rx).run(&mut terminal);
    ratatui::restore();

    app_result

    /*
    for received in rx {
        match received {
            pwmixer::message::Message::Monitor(message) => println!("{:?}", message),
            message => println!("{:?}", message),
        }
    }

    Ok(())
    */
}
