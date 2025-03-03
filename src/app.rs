//! Main rendering and event processing for the application.

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
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, MouseButton,
    MouseEvent, MouseEventKind,
};

use crate::command::Command;
use crate::device_type::DeviceType;
use crate::event::Event;
use crate::object::ObjectId;
use crate::object_list::{ObjectList, ObjectListWidget};
use crate::state::{State, StateDirty};
use crate::view::{self, ListType, View};

#[cfg(feature = "trace")]
use crate::{trace, trace_dbg};

#[derive(Clone, Copy)]
pub enum Action {
    SelectTab(usize),
    ScrollUp,
    ScrollDown,
    OpenPopup,
    ClosePopup,
    SelectObject(ObjectId),
    SetTarget(view::Target),
    ToggleMute,
    SetAbsoluteVolume(f32),
    SetRelativeVolume(f32),
    SetDefault,
}

struct Tab {
    title: String,
    list: ObjectList,
}

impl Tab {
    fn new(title: String, list: ObjectList) -> Self {
        Self { title, list }
    }
}

// Mouse events matching one of the MouseEventKinds within the Rect will
// perform the Actions.
pub type MouseArea = (Rect, Vec<MouseEventKind>, Vec<Action>);

pub struct App {
    exit: bool,
    tx: pipewire::channel::Sender<Command>,
    rx: mpsc::Receiver<Event>,
    error_message: Option<String>,
    tabs: Vec<Tab>,
    selected_tab_index: usize,
    mouse_areas: Vec<MouseArea>,
    /// The monitor has received all initial information.
    is_ready: bool,
    state: State,
    view: View,
}

impl App {
    pub fn new(
        tx: pipewire::channel::Sender<Command>,
        rx: mpsc::Receiver<Event>,
    ) -> Self {
        let tabs = vec![
            Tab::new(
                String::from("Playback"),
                ObjectList::new(ListType::Node(view::NodeType::Playback), None),
            ),
            Tab::new(
                String::from("Recording"),
                ObjectList::new(
                    ListType::Node(view::NodeType::Recording),
                    None,
                ),
            ),
            Tab::new(
                String::from("Output Devices"),
                ObjectList::new(
                    ListType::Node(view::NodeType::Output),
                    Some(DeviceType::Sink),
                ),
            ),
            Tab::new(
                String::from("Input Devices"),
                ObjectList::new(
                    ListType::Node(view::NodeType::Input),
                    Some(DeviceType::Source),
                ),
            ),
            Tab::new(
                String::from("Configuration"),
                ObjectList::new(ListType::Device, None),
            ),
        ];
        App {
            exit: Default::default(),
            tx,
            rx,
            error_message: Default::default(),
            tabs,
            selected_tab_index: Default::default(),
            mouse_areas: Default::default(),
            is_ready: Default::default(),
            state: Default::default(),
            view: Default::default(),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            self.mouse_areas.clear();

            // Update view if needed
            match self.state.dirty {
                StateDirty::Everything => {
                    self.view = View::from(&self.state);
                }
                StateDirty::PeaksOnly => {
                    self.view.update_peaks(&self.state);
                }
                _ => {}
            }
            self.state.dirty = StateDirty::Clean;

            #[cfg(feature = "trace")]
            trace_dbg!(&self.view);

            if self.is_ready
                && self.tabs[self.selected_tab_index].list.selected.is_none()
            {
                self.handle_action(Action::ScrollDown);
            }

            terminal.draw(|frame| {
                self.tabs[self.selected_tab_index]
                    .list
                    .update(frame.area(), &self.view);

                self.draw(frame);
            })?;
            self.handle_events()?;
        }

        self.error_message.map_or(Ok(()), |s| Err(anyhow!(s)))
    }

    fn draw(&mut self, frame: &mut Frame) {
        let widget = AppWidget {
            selected_tab_index: self.selected_tab_index,
            view: &self.view,
        };
        let mut widget_state = AppWidgetState {
            mouse_areas: &mut self.mouse_areas,
            tabs: &mut self.tabs,
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
            for command in self.state.update(event) {
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
            CrosstermEvent::Mouse(mouse_event) => {
                self.handle_mouse_event(mouse_event)
            }
            _ => (),
        };

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('m') => {
                self.handle_action(Action::ToggleMute);
            }
            KeyCode::Char('d') => {
                self.handle_action(Action::SetDefault);
            }
            KeyCode::Char('l') => {
                self.handle_action(Action::SetRelativeVolume(0.01));
            }
            KeyCode::Char('h') => {
                self.handle_action(Action::SetRelativeVolume(-0.01));
            }
            KeyCode::Char('q') => self.exit(None),
            KeyCode::Char('c') => {
                self.handle_action(Action::OpenPopup);
            }
            KeyCode::Esc => self.handle_action(Action::ClosePopup),
            KeyCode::Enter => {
                let commands = self.tabs[self.selected_tab_index]
                    .list
                    .popup_select(&self.view);
                for command in commands {
                    let _ = self.tx.send(command);
                }
            }
            KeyCode::Char('j') => {
                self.handle_action(Action::ScrollDown);
            }
            KeyCode::Char('k') => {
                self.handle_action(Action::ScrollUp);
            }
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
        let actions = self
            .mouse_areas
            .iter()
            .rev()
            .find(|(rect, kinds, _)| {
                rect.contains(Position {
                    x: mouse_event.column,
                    y: mouse_event.row,
                }) && kinds.contains(&mouse_event.kind)
            })
            .map(|(_, _, action)| action.clone())
            .into_iter()
            .flatten();

        for action in actions {
            self.handle_action(action);
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::SelectTab(index) => self.selected_tab_index = index,
            Action::ScrollDown => {
                self.tabs[self.selected_tab_index].list.down(&self.view);
            }
            Action::ScrollUp => {
                self.tabs[self.selected_tab_index].list.up(&self.view);
            }
            Action::OpenPopup => {
                self.tabs[self.selected_tab_index]
                    .list
                    .popup_open(&self.view);
            }
            Action::ClosePopup => {
                self.tabs[self.selected_tab_index].list.popup_close();
            }
            Action::SetTarget(target) => {
                let commands = self.tabs[self.selected_tab_index]
                    .list
                    .set_target(&self.view, target);
                for command in commands {
                    let _ = self.tx.send(command);
                }
            }
            Action::SelectObject(object_id) => {
                self.tabs[self.selected_tab_index].list.selected =
                    Some(object_id)
            }
            Action::ToggleMute => {
                let commands = self.tabs[self.selected_tab_index]
                    .list
                    .toggle_mute(&self.view);
                for command in commands {
                    let _ = self.tx.send(command);
                }
            }
            Action::SetAbsoluteVolume(volume) => {
                let commands = self.tabs[self.selected_tab_index]
                    .list
                    .set_absolute_volume(&self.view, volume);
                for command in commands {
                    let _ = self.tx.send(command);
                }
            }
            Action::SetRelativeVolume(volume) => {
                let commands = self.tabs[self.selected_tab_index]
                    .list
                    .set_relative_volume(&self.view, volume);
                for command in commands {
                    let _ = self.tx.send(command);
                }
            }
            Action::SetDefault => {
                let commands = self.tabs[self.selected_tab_index]
                    .list
                    .set_default(&self.view);
                for command in commands {
                    let _ = self.tx.send(command);
                }
            }
        }
    }
}

pub struct AppWidget<'a> {
    selected_tab_index: usize,
    view: &'a View,
}

pub struct AppWidgetState<'a> {
    mouse_areas: &'a mut Vec<MouseArea>,
    tabs: &'a mut Vec<Tab>,
}

impl<'a> StatefulWidget for AppWidget<'a> {
    type State = AppWidgetState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // list_area
                Constraint::Length(1), // menu_area
            ])
            .split(area);
        let list_area = layout[0];
        let menu_area = layout[1];

        let constraints: Vec<_> = state
            .tabs
            .iter()
            .map(|tab| Constraint::Length(tab.title.len() as u16 + 2))
            .collect();

        let menu_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .split(menu_area);

        for (i, tab) in state.tabs.iter().enumerate() {
            let (title, style) = if i == self.selected_tab_index {
                (
                    format!("[{}]", tab.title),
                    Style::default().fg(Color::LightCyan),
                )
            } else {
                (format!(" {} ", tab.title), Style::default())
            };
            Line::from(Span::styled(title, style)).render(menu_areas[i], buf);

            state.mouse_areas.push((
                menu_areas[i],
                vec![MouseEventKind::Down(MouseButton::Left)],
                vec![Action::SelectTab(i)],
            ));
        }

        let mut widget = ObjectListWidget {
            object_list: &mut state.tabs[self.selected_tab_index].list,
            view: self.view,
        };
        widget.render(list_area, buf, state.mouse_areas);
    }
}
