use std::io::stdout;
use std::sync::{mpsc, Arc};

use anyhow::Result;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    ExecutableCommand,
};

use wiremix::app;
use wiremix::config::Config;
use wiremix::event::Event;
use wiremix::input;
use wiremix::monitor;
use wiremix::opt::Opt;

fn main() -> Result<()> {
    // Event channel for sending PipeWire and input events to the UI
    let (event_tx, event_rx) = mpsc::channel();
    let event_tx = Arc::new(event_tx);

    // Parse command-line arguments
    let opt = Opt::parse();

    let config_default_path = Config::default_path();
    let config_path = opt.config.as_deref().or(config_default_path.as_deref());

    let config = Config::try_new(config_path, &opt)?;

    // Handler for events from the PipeWire monitor - just wrap them and put
    // them on the event channel.
    let event_handler = {
        let event_tx = Arc::clone(&event_tx);
        move |event| event_tx.send(Event::Monitor(event)).is_ok()
    };
    // Spawn the PipeWire monitor
    let monitor_handle = monitor::spawn(config.remote.clone(), event_handler)?;
    let _input_handle = input::spawn(Arc::clone(&event_tx));

    #[cfg(debug_assertions)]
    if opt.dump_events {
        // Event dumping mode for debugging the monitor code
        for received in event_rx {
            use wiremix::event::Event;
            match received {
                Event::Monitor(event) => print!("{event:?}\r\n"),
                event => {
                    print!("{event:?}\r\n");
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
        app::App::new(&monitor_handle.tx, event_rx, config).run(&mut terminal);
    ratatui::restore();
    if support_mouse {
        stdout().execute(DisableMouseCapture)?;
    }

    app_result
}
