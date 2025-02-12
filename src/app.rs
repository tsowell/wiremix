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
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, MouseEvent,
    MouseEventKind,
};

use crate::command::Command;
use crate::device_type::DeviceType;
use crate::event::Event;
use crate::media_class::MediaClass;
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
    tx: pipewire::channel::Sender<Command>,
    rx: mpsc::Receiver<Event>,
    error_message: Option<String>,
    tabs: Vec<Tab>,
    selected_tab_index: usize,
    click_areas: Vec<(Rect, Action)>,
    /// The monitor has received all initial information.
    is_ready: bool,
}

impl App {
    pub fn new(
        tx: pipewire::channel::Sender<Command>,
        rx: mpsc::Receiver<Event>,
    ) -> Self {
        let mut tabs = Vec::new();
        tabs.push(Tab::new(
            String::from("Playback"),
            NodeList::new(
                Box::new(|node| {
                    node.media_class
                        .as_ref()
                        .is_some_and(MediaClass::is_sink_input)
                }),
                None,
            ),
        ));
        tabs.push(Tab::new(
            String::from("Recording"),
            NodeList::new(
                Box::new(|node| {
                    node.media_class
                        .as_ref()
                        .is_some_and(MediaClass::is_source_output)
                }),
                None,
            ),
        ));
        tabs.push(Tab::new(
            String::from("Output Devices"),
            NodeList::new(
                Box::new(|node| {
                    node.media_class.as_ref().is_some_and(MediaClass::is_sink)
                }),
                Some(DeviceType::Sink),
            ),
        ));
        tabs.push(Tab::new(
            String::from("Input Devices"),
            NodeList::new(
                Box::new(|node| {
                    node.media_class.as_ref().is_some_and(MediaClass::is_source)
                }),
                Some(DeviceType::Source),
            ),
        ));
        tabs.push(Tab::new(
            String::from("Configuration"),
            /* TODO - for now just show all nodes */
            NodeList::new(Box::new(|_node| true), None),
        ));
        App {
            exit: Default::default(),
            tx,
            rx,
            error_message: Default::default(),
            tabs,
            selected_tab_index: Default::default(),
            click_areas: Default::default(),
            is_ready: Default::default(),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            self.click_areas.clear();
            terminal.draw(|frame| {
                self.tabs[self.selected_tab_index]
                    .list
                    .update(frame.area(), self.is_ready);
                self.draw(frame);
            })?;
            self.handle_events()?;
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

    fn handle_events(&mut self) -> Result<()> {
        // Block on getting the next event.
        self.handle_event(self.rx.recv()?)?;
        // Then handle the rest that are available.
        while let Ok(event) = self.rx.try_recv() {
            self.handle_event(event)?;
        }

        Ok(())
    }

    fn handle_event(&mut self, event: Event) -> Result<()> {
        #[cfg(feature = "trace")]
        trace_dbg!(&event);

        if let Event::Input(event) = event {
            self.handle_input_event(event)
        } else if let Event::Error(error) = event {
            match error {
                // These happen when objects are removed while the monitor is
                // still in the process of setting up listeners.
                error if error.starts_with("no global ") => {}
                error if error.starts_with("unknown resource ") => {}
                // I see this one when disconnecting a Bluetooth sink.
                error if error == "Received error event" => {}
                _ => self.exit(Some(error)),
            }
            Ok(())
        } else if let Event::Ready = event {
            self.is_ready = true;
            Ok(())
        } else if let Event::Monitor(event) = event {
            for command in STATE.with_borrow_mut(|s| s.update(event)) {
                let _ = self.tx.send(command);
            }

            Ok(())
        } else {
            Ok(())
        }
    }

    fn handle_input_event(&mut self, event: CrosstermEvent) -> Result<()> {
        match event {
            CrosstermEvent::Key(key_event)
                if key_event.kind == KeyEventKind::Press =>
            {
                self.handle_key_event(key_event)
            }
            CrosstermEvent::Mouse(
                mouse_event @ MouseEvent {
                    kind: MouseEventKind::Down(_),
                    ..
                },
            ) => self.handle_mouse_event(mouse_event),
            _ => (),
        };

        Ok(())
    }

    fn selected_list(&mut self) -> &mut NodeList {
        &mut self.tabs[self.selected_tab_index].list
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('m') => {
                if let Some(command) = self.selected_list().mute() {
                    let _ = self.tx.send(command);
                }
            }
            KeyCode::Char('d') => {
                if let Some(command) = self.selected_list().set_default() {
                    let _ = self.tx.send(command);
                }
            }
            KeyCode::Char('l') => {
                if let Some(command) =
                    self.selected_list().volume(|volume| volume + 0.01)
                {
                    let _ = self.tx.send(command);
                }
            }
            KeyCode::Char('h') => {
                if let Some(command) =
                    self.selected_list().volume(|volume| volume - 0.01)
                {
                    let _ = self.tx.send(command);
                }
            }
            KeyCode::Char('q') => self.exit(None),
            KeyCode::Char('j') => self.selected_list().down(),
            KeyCode::Char('k') => self.selected_list().up(),
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
            let (title, style) = if i == self.selected_tab_index {
                (tab.title.to_uppercase(), Style::default().fg(Color::Green))
            } else {
                (tab.title.clone(), Style::default())
            };
            Line::from(Span::styled(title, style)).render(menu_areas[i], buf);

            state
                .click_areas
                .push((menu_areas[i], Action::SelectTab(i)));
        }

        self.tabs[self.selected_tab_index]
            .list
            .render(list_area, buf);
    }
}
