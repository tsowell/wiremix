use std::cell::RefCell;
use std::sync::mpsc;

use anyhow::{anyhow, Result};

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget},
    DefaultTerminal, Frame,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::message::{InputMessage, Message};
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
}

impl App {
    pub fn new(rx: mpsc::Receiver<Message>) -> Self {
        App {
            exit: Default::default(),
            rx,
            log: Default::default(),
            error_message: Default::default(),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
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
            _ => (),
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        STATE.with_borrow(|state| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(state.nodes.iter().map(|_| Constraint::Length(5)))
                .split(area);
            for (node, area) in state.nodes.values().zip(layout.iter()) {
                node.render(*area, buf);
            }
        })
    }
}

impl Widget for &state::Node {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_block = Block::default().borders(Borders::ALL);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([1, 1, 1].iter().map(|&c| Constraint::Length(c)))
            .split(border_block.inner(area));
        border_block.render(area, buf);

        let header = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(80), Constraint::Min(0)])
            .split(layout[0]);

        let left = if let Some(description) = &self.description {
            description.clone()
        } else if let (Some(name), Some(media_name)) =
            (&self.name, &self.media_name)
        {
            format!("{}: {}", name, media_name).to_string()
        } else {
            "".to_string()
        };

        Line::from(left).render(header[0], buf);

        if let Some(volumes) = &self.volumes {
            if !volumes.is_empty() {
                let mean = volumes.iter().sum::<f32>() / volumes.len() as f32;
                let volume = mean.cbrt();

                let count = (volume * area.width as f32) as usize;
                let percent = (volume * 100.0) as u32;
                Paragraph::new(format!("{}| {}%", " ".repeat(count), percent))
                    .render(layout[1], buf);
            }
        }

        if let Some(peak) = &self.peak {
            let count = (peak * area.width as f32) as usize;
            Paragraph::new("=".repeat(count).to_string())
                .render(layout[2], buf);
        }
    }
}
