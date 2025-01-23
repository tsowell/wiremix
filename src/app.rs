use std::cell::RefCell;
use std::sync::mpsc;

use anyhow::{anyhow, Result};

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
    DefaultTerminal, Frame,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::message::{InputMessage, Message};
use crate::named_constraints::with_named_constraints;
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
    tabs: Vec<(String, Alignment, NodeList)>,
    selected_tab_index: usize,
}

impl App {
    pub fn new(rx: mpsc::Receiver<Message>) -> Self {
        let mut tabs = Vec::new();
        tabs.push((
            String::from("Playback"),
            Alignment::Left,
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Stream/Output/Audio"))
            })),
        ));
        tabs.push((
            String::from("Recording"),
            Alignment::Left,
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Stream/Input/Audio"))
            })),
        ));
        tabs.push((
            String::from("Output Devices"),
            Alignment::Center,
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Audio/Sink"))
            })),
        ));
        tabs.push((
            String::from("Input Devices"),
            Alignment::Right,
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Audio/Source"))
            })),
        ));
        tabs.push((
            String::from("Configuration"),
            Alignment::Right,
            /* TODO - for now just show all nodes */
            NodeList::new(Box::new(|_node| true)),
        ));
        App {
            exit: Default::default(),
            rx,
            log: Default::default(),
            error_message: Default::default(),
            tabs,
            selected_tab_index: Default::default(),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            terminal.draw(|frame| {
                self.tabs[self.selected_tab_index].2.update(frame.area());
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
            KeyCode::Char('j') => self.tabs[self.selected_tab_index].2.down(),
            KeyCode::Char('k') => self.tabs[self.selected_tab_index].2.up(),
            KeyCode::Char('H') => {
                self.selected_tab_index =
                    self.selected_tab_index.checked_sub(1).unwrap_or(4)
            }
            KeyCode::Char('L') => {
                self.selected_tab_index = (self.selected_tab_index + 1) % 5
            }
            _ => (),
        }
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut list_area = Default::default();
        let mut menu_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Min(0), Some(&mut list_area)),
                (Constraint::Length(1), Some(&mut menu_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(area)
            }
        );

        let menu_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Min(0),
                Constraint::Min(0),
                Constraint::Min(0),
                Constraint::Min(0),
                Constraint::Min(0),
            ])
            .split(menu_area);

        for (i, (title, alignment, _)) in self.tabs.iter().enumerate() {
            let style = if i == self.selected_tab_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            Line::from(Span::styled(title, style))
                .alignment(*alignment)
                .render(menu_areas[i], buf);
        }

        self.tabs[self.selected_tab_index].2.render(list_area, buf);
    }
}
