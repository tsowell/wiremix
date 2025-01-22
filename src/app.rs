use std::cell::RefCell;
use std::sync::mpsc;

use anyhow::{anyhow, Result};

use ratatui::{
    prelude::{Buffer, Rect},
    widgets::Widget,
    DefaultTerminal, Frame,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::message::{InputMessage, Message};
use crate::node_list::NodeList;
use crate::state;

#[cfg(feature = "trace")]
use crate::{trace, trace_dbg};

thread_local! {
    pub static STATE: RefCell<state::State> = RefCell::new(Default::default());
}

pub struct App {
    exit: bool,
    rx: mpsc::Receiver<Message>,
    log: Vec<String>,
    error_message: Option<String>,
    node_list: NodeList,
}

impl App {
    pub fn new(rx: mpsc::Receiver<Message>) -> Self {
        App {
            exit: Default::default(),
            rx,
            log: Default::default(),
            error_message: Default::default(),
            node_list: NodeList::new(Box::new(|_node| true)),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            terminal.draw(|frame| {
                self.node_list.update(frame.area());
                self.draw(frame)
            })?;
            self.handle_messages()?;
        }

        self.error_message.map_or(Ok(()), |s| Err(anyhow!(s)))
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn exit(&mut self, error_message: Option<String>) {
        self.exit = true;
        self.error_message = error_message;
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
        } else if let Message::Error(error) = message {
            self.exit(Some(error));
            Ok(())
        } else if let Message::Monitor(message) = message {
            self.log.push(format!("{:?}", message));
            STATE.with_borrow_mut(|s| s.update(message));
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
            KeyCode::Char('q') => self.exit(None),
            KeyCode::Char('j') => self.node_list.down(),
            KeyCode::Char('k') => self.node_list.up(),
            _ => (),
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.node_list.render(area, buf);
    }
}
