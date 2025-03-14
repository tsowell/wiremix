use std::io::stdout;
use std::sync::{mpsc, Arc};

use anyhow::Result;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    ExecutableCommand,
};

use pwmixer::app;
use pwmixer::command::Command;
use pwmixer::config::Config;
use pwmixer::input;
use pwmixer::monitor;
use pwmixer::opt::Opt;

fn main() -> Result<()> {
    // Event channel for sending PipeWire and input events to the UI
    let (event_tx, event_rx) = mpsc::channel();
    let event_tx = Arc::new(event_tx);

    // Command channel for the UI to send commands to control PipeWire
    let (command_tx, command_rx) = pipewire::channel::channel::<Command>();

    // Parse command-line arguments
    let opt = Opt::parse();

    let config_default_path = Config::default_path();
    let config_path = opt.config.as_deref().or(config_default_path.as_deref());

    let config = Config::try_new(config_path, &opt)?;

    // Spawn the PipeWire monitor
    let _monitor_handle = monitor::spawn(
        config.remote.clone(),
        Arc::clone(&event_tx),
        command_rx,
    )?;
    let _input_handle = input::spawn(Arc::clone(&event_tx));

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
    let support_mouse = config.mouse;
    if support_mouse {
        stdout().execute(EnableMouseCapture)?;
    }
    let mut terminal = ratatui::init();
    let app_result =
        app::App::new(command_tx, event_rx, config).run(&mut terminal);
    ratatui::restore();
    if support_mouse {
        stdout().execute(DisableMouseCapture)?;
    }

    app_result
}
