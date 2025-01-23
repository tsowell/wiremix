use std::cell::RefCell;
use std::sync::mpsc;

use anyhow::{anyhow, Result};

use ratatui::{
    prelude::{Buffer, Constraint, Direction, Layout, Position, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
    DefaultTerminal, Frame,
};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind,
};

use crate::message::{InputMessage, Message};
use crate::named_constraints::with_named_constraints;
use crate::node_list::NodeList;
use crate::state;

#[cfg(feature = "trace")]
use crate::{trace, trace_dbg};

thread_local! {
    pub static STATE: RefCell<state::State> = RefCell::new(Default::default());
}

#[derive(Clone)]
pub enum Action {
    SelectTab(usize),
}

struct Tab {
    title: String,
    list: NodeList,
}

impl Tab {
    fn new(title: String, list: NodeList) -> Self {
        Self { title, list }
    }
}

pub struct App {
    exit: bool,
    rx: mpsc::Receiver<Message>,
    log: Vec<String>,
    error_message: Option<String>,
    tabs: Vec<Tab>,
    selected_tab_index: usize,
    click_areas: Vec<(Rect, Action)>,
}

impl App {
    pub fn new(rx: mpsc::Receiver<Message>) -> Self {
        let mut tabs = Vec::new();
        tabs.push(Tab::new(
            String::from("Playback"),
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Stream/Output/Audio"))
            })),
        ));
        tabs.push(Tab::new(
            String::from("Recording"),
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Stream/Input/Audio"))
            })),
        ));
        tabs.push(Tab::new(
            String::from("Output Devices"),
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Audio/Sink"))
            })),
        ));
        tabs.push(Tab::new(
            String::from("Input Devices"),
            NodeList::new(Box::new(|node| {
                node.media_class == Some(String::from("Audio/Source"))
            })),
        ));
        tabs.push(Tab::new(
            String::from("Configuration"),
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
            click_areas: Default::default(),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            self.click_areas.clear();
            terminal.draw(|frame| {
                self.tabs[self.selected_tab_index].list.update(frame.area());
                self.draw(frame);
            })?;
            self.handle_messages()?;
        }

        self.error_message.map_or(Ok(()), |s| Err(anyhow!(s)))
    }

    fn draw(&mut self, frame: &mut Frame) {
        let widget = AppWidget {
            tabs: &self.tabs,
            selected_tab_index: self.selected_tab_index,
        };
        let mut widget_state = AppWidgetState {
            click_areas: &mut self.click_areas,
        };
        frame.render_stateful_widget(widget, frame.area(), &mut widget_state);
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
            match error {
                error if error.starts_with("no global ") => {}
                _ => self.exit(Some(error)),
            }
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
            Event::Mouse(
                mouse_event @ MouseEvent {
                    kind: MouseEventKind::Down(_),
                    ..
                },
            ) => self.handle_mouse_event(mouse_event),
            _ => (),
        };

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(None),
            KeyCode::Char('j') => {
                self.tabs[self.selected_tab_index].list.down()
            }
            KeyCode::Char('k') => self.tabs[self.selected_tab_index].list.up(),
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

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        let action = self
            .click_areas
            .iter()
            .rev()
            .find(|(rect, _)| {
                rect.contains(Position {
                    x: mouse_event.column,
                    y: mouse_event.row,
                })
            })
            .map(|(_, action)| action);

        if let Some(action) = action {
            self.handle_action(action.clone());
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::SelectTab(index) => self.selected_tab_index = index,
        }
    }
}

pub struct AppWidget<'a> {
    tabs: &'a Vec<Tab>,
    selected_tab_index: usize,
}

pub struct AppWidgetState<'a> {
    click_areas: &'a mut Vec<(Rect, Action)>,
}

impl<'a> StatefulWidget for AppWidget<'a> {
    type State = AppWidgetState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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

        let mut constraints: Vec<Constraint> = Default::default();
        for tab in self.tabs.iter() {
            constraints.push(Constraint::Length(tab.title.len() as u16));
        }

        let menu_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .spacing(2)
            .split(menu_area);

        for (i, tab) in self.tabs.iter().enumerate() {
            let style = if i == self.selected_tab_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            Line::from(Span::styled(tab.title.clone(), style))
                .render(menu_areas[i], buf);

            state
                .click_areas
                .push((menu_areas[i], Action::SelectTab(i)));
        }

        self.tabs[self.selected_tab_index]
            .list
            .render(list_area, buf);
    }
}
