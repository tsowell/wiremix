// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

use anyhow::Result;
use clap::Parser;
use pwmixer::monitor::monitor_pipewire;

use std::sync::mpsc;
use std::thread;

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

    thread::spawn(move || {
        let opt = Opt::parse();
        let _ = monitor_pipewire(opt.remote, tx, !opt.no_capture);
    });

    for received in rx {
        println!("{:?}", received);
    }

    Ok(())
}
