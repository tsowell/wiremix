// Copyright The pipewire-rs Contributors.
// SPDX-License-Identifier: MIT

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

#[derive(Parser)]
#[clap(name = "pwmixer", about = "PipeWire mixer")]
struct Opt {
    #[clap(short, long, help = "The name of the remote to connect to")]
    remote: Option<String>,

    // TODO
    #[clap(short, long, help = "Disable audio capture for level monitoring")]
    no_capture: bool,

    #[clap(short, long, help = "Dump events without showing interface")]
    dump_events: bool,
}

fn main() -> Result<()> {
    let (event_tx, event_rx) = mpsc::channel();
    let event_tx = Arc::new(event_tx);

    let (command_tx, command_rx) = pipewire::channel::channel::<Command>();

    let opt = Opt::parse();
    let _monitor_handle =
        monitor::spawn(opt.remote, Arc::clone(&event_tx), command_rx)?;
    let _input_handle = input::spawn(Arc::clone(&event_tx));

    if opt.dump_events {
        //stdout().execute(EnableMouseCapture)?;
        crossterm::terminal::enable_raw_mode()?;
        let _guard = scopeguard::guard((), |_| {
            let _ = crossterm::terminal::disable_raw_mode();
            //let _ = stdout().execute(DisableMouseCapture);
        });
        for received in event_rx {
            use crossterm::event::{
                Event as CrosstermEvent, KeyCode, KeyEvent,
            };
            use pwmixer::event::Event;
            match received {
                Event::Monitor(event) => print!("{:?}\r\n", event),
                Event::Input(CrosstermEvent::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                })) => break,
                event => {
                    print!("{:?}\r\n", event);
                }
            }
        }

        Ok(())
    } else {
        stdout().execute(EnableMouseCapture)?;
        let mut terminal = ratatui::init();
        let app_result = app::App::new(command_tx, event_rx).run(&mut terminal);
        ratatui::restore();
        stdout().execute(DisableMouseCapture)?;

        app_result
    }
}
