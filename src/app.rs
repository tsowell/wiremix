use std::sync::mpsc;

use anyhow::Result;

use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Paragraph, Widget},
    DefaultTerminal, Frame,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::message::{InputMessage, Message};
use crate::state::State;

#[cfg(feature = "trace")]
use crate::{trace, trace_dbg};

pub struct App {
    exit: bool,
    rx: mpsc::Receiver<Message>,
    log: Vec<String>,
    state: State,
}

impl App {
    pub fn new(rx: mpsc::Receiver<Message>) -> Self {
        App {
            exit: Default::default(),
            rx,
            log: Default::default(),
            state: Default::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_messages()?;

            #[cfg(feature = "trace")]
            trace_dbg!(&self.state);
        }

        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn handle_messages(&mut self) -> Result<()> {
        // Block on getting the next message.
        self.handle_message(self.rx.recv()?)?;
        // Then handle the rest that are available.
        while let Ok(message) = self.rx.try_recv() {
            self.handle_message(message)?;
        }

        Ok(())
    }

    fn handle_message(&mut self, message: Message) -> Result<()> {
        if let Message::Input(InputMessage::Event(event)) = message {
            self.handle_event(event)
        } else if let Message::Quit = message {
            self.exit();
            Ok(())
        } else if let Message::Monitor(message) = message {
            self.log.push(format!("{:?}", message));
            self.state.update(message);
            Ok(())
        } else {
            Ok(())
        }
    }

    fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => (),
        };

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => (),
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let log = &self.log;
        let scroll = (log.len().saturating_sub(area.height as usize)) as u16;
        Paragraph::new(log.join("\n"))
            .scroll((scroll, 0))
            .render(area, buf);
    }
}
