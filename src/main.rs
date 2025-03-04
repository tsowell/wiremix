use std::io::stdout;
use std::sync::{mpsc, Arc};

use anyhow::Result;
use clap::Parser;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    ExecutableCommand,
};

use pwmixer::app;
use pwmixer::command::Command;
use pwmixer::input;
use pwmixer::monitor;
use pwmixer::vsync;

#[derive(Parser)]
#[clap(name = "pwmixer", about = "PipeWire mixer")]
struct Opt {
    #[clap(short, long, help = "The name of the remote to connect to")]
    remote: Option<String>,

    // TODO
    #[clap(short, long, help = "Disable audio capture for level monitoring")]
    no_capture: bool,

    #[clap(short, long, help = "Target frames per second")]
    fps: Option<f32>,

    #[cfg(debug_assertions)]
    #[clap(short, long, help = "Dump events without showing interface")]
    dump_events: bool,
}

fn main() -> Result<()> {
    // Event channel for sending PipeWire and input events to the UI
    let (event_tx, event_rx) = mpsc::channel();
    let event_tx = Arc::new(event_tx);

    // Command channel for the UI to send commands to control PipeWire
    let (command_tx, command_rx) = pipewire::channel::channel::<Command>();

    let opt = Opt::parse();

    // Spawn the PipeWire monitor
    let _monitor_handle =
        monitor::spawn(opt.remote, Arc::clone(&event_tx), command_rx)?;
    let _input_handle = input::spawn(Arc::clone(&event_tx));
    let _vsync_handle =
        opt.fps.map(|fps| vsync::spawn(Arc::clone(&event_tx), fps));

    #[cfg(debug_assertions)]
    if opt.dump_events {
        // Event dumping mode for debugging the monitor code
        for received in event_rx {
            use pwmixer::event::Event;
            match received {
                Event::Monitor(event) => print!("{:?}\r\n", event),
                event => {
                    print!("{:?}\r\n", event);
                }
            }
        }

        return Ok(());
    }

    // Normal UI mode
    stdout().execute(EnableMouseCapture)?;
    let mut terminal = ratatui::init();
    let app_result = app::App::new(command_tx, event_rx, opt.fps.is_some())
        .run(&mut terminal);
    ratatui::restore();
    stdout().execute(DisableMouseCapture)?;

    app_result
}
