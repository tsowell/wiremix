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
use crate::named_constraints::with_named_constraints;
use crate::object::ObjectId;
use crate::object_list::{ObjectList, ObjectListWidget};
use crate::state::State;
use crate::view::{self, ListType, View, VolumeAdjustment};

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
    AbsoluteVolume(f32),
    RelativeVolume(f32),
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

            self.view = View::from(&self.state);
            #[cfg(feature = "trace")]
            trace_dbg!(&self.view);

            if self.is_ready && self.selected_list().selected.is_none() {
                let new_selected = {
                    let selected_list = self.selected_list();
                    let list_type = selected_list.list_type;
                    self.view.next_id(list_type, None)
                };
                if new_selected.is_some() {
                    self.selected_list_mut().selected = new_selected;
                }
            }

            terminal.draw(|frame| {
                let list_type = self.selected_list().list_type;
                let selected_index =
                    self.selected_list().selected.and_then(|selected| {
                        self.view.position(list_type, selected)
                    });
                let len = self.view.len(list_type);
                self.selected_list_mut().update(
                    frame.area(),
                    selected_index,
                    len,
                );
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
            self.view = View::from(&self.state);
            #[cfg(feature = "trace")]
            trace_dbg!(&self.view);

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

    fn selected_list(&self) -> &ObjectList {
        &self.tabs[self.selected_tab_index].list
    }

    fn selected_list_mut(&mut self) -> &mut ObjectList {
        &mut self.tabs[self.selected_tab_index].list
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('m') if self.selected_list().list_type.is_node() => {
                self.handle_action(Action::ToggleMute);
            }
            KeyCode::Char('d') if self.selected_list().list_type.is_node() => {
                self.handle_action(Action::SetDefault);
            }
            KeyCode::Char('l') if self.selected_list().list_type.is_node() => {
                self.handle_action(Action::RelativeVolume(0.01));
            }
            KeyCode::Char('h') if self.selected_list().list_type.is_node() => {
                self.handle_action(Action::RelativeVolume(-0.01));
            }
            KeyCode::Char('q') => self.exit(None),
            KeyCode::Char('c') => {
                self.handle_action(Action::OpenPopup);
            }
            KeyCode::Esc => self.handle_action(Action::ClosePopup),
            KeyCode::Enter => {
                let selected_list = self.selected_list();
                let commands = selected_list
                    .selected
                    .zip(selected_list.selected_target())
                    .map(|(object_id, &target)| {
                        self.view.set_target(object_id, target)
                    })
                    .into_iter()
                    .flatten();
                for command in commands {
                    let _ = self.tx.send(command);
                }
                self.selected_list_mut().list_state.select(None);
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
            Action::ScrollDown => self.action_scroll_down(),
            Action::ScrollUp => self.action_scroll_up(),
            Action::OpenPopup => self.action_open_popup(),
            Action::ClosePopup => {
                self.selected_list_mut().list_state.select(None)
            }
            Action::SetTarget(target) => self.action_set_target(target),
            Action::SelectObject(object_id) => {
                self.selected_list_mut().selected = Some(object_id)
            }
            Action::ToggleMute => self.action_toggle_mute(),
            Action::AbsoluteVolume(volume) => {
                self.action_absolute_volume(volume)
            }
            Action::RelativeVolume(volume) => {
                self.action_relative_volume(volume)
            }
            Action::SetDefault => self.action_set_default(),
        }
    }

    fn action_scroll_down(&mut self) {
        let selected_list = self.selected_list();
        if selected_list.list_state.selected().is_some() {
            self.selected_list_mut().list_state.select_next();
        } else {
            let new_selected = {
                let selected = selected_list.selected;
                let list_type = selected_list.list_type;
                self.view.next_id(list_type, selected)
            };
            if new_selected.is_some() {
                self.selected_list_mut().selected = new_selected;
            }
        }
    }

    fn action_scroll_up(&mut self) {
        let selected_list = self.selected_list();
        if selected_list.list_state.selected().is_some() {
            self.selected_list_mut().list_state.select_previous();
        } else {
            let new_selected = {
                let selected = selected_list.selected;
                let list_type = selected_list.list_type;
                self.view.previous_id(list_type, selected)
            };
            if new_selected.is_some() {
                self.selected_list_mut().selected = new_selected;
            }
        }
    }

    fn action_open_popup(&mut self) {
        let targets = match self.selected_list().list_type {
            ListType::Node(_) => self
                .selected_list()
                .selected
                .and_then(|object_id| self.view.node_targets(object_id)),
            ListType::Device => self
                .selected_list()
                .selected
                .and_then(|object_id| self.view.device_targets(object_id)),
        };
        if let Some((targets, index)) = targets {
            if !targets.is_empty() {
                let selected_list = self.selected_list_mut();
                selected_list.targets = targets;
                selected_list.list_state.select(Some(index));
            }
        }
    }

    fn action_set_target(&mut self, target: view::Target) {
        self.selected_list_mut().list_state.select(None);
        let commands = self
            .selected_list()
            .selected
            .map(|object_id| self.view.set_target(object_id, target))
            .into_iter()
            .flatten();
        for command in commands {
            let _ = self.tx.send(command);
        }
    }

    fn action_toggle_mute(&mut self) {
        let command = self
            .selected_list()
            .selected
            .and_then(|node_id| self.view.mute(node_id));
        if let Some(command) = command {
            let _ = self.tx.send(command);
        }
    }

    fn action_absolute_volume(&mut self, volume: f32) {
        let command = self.selected_list().selected.and_then(|node_id| {
            self.view
                .volume(node_id, VolumeAdjustment::Absolute(volume))
        });
        if let Some(command) = command {
            let _ = self.tx.send(command);
        }
    }

    fn action_relative_volume(&mut self, volume: f32) {
        let command = self.selected_list().selected.and_then(|node_id| {
            self.view
                .volume(node_id, VolumeAdjustment::Relative(volume))
        });
        if let Some(command) = command {
            let _ = self.tx.send(command);
        }
    }

    fn action_set_default(&mut self) {
        let node_id = self.selected_list().selected;
        let device_type = self.selected_list().device_type;
        let command =
            node_id.zip(device_type).and_then(|(node_id, device_type)| {
                self.view.set_default(node_id, device_type)
            });
        if let Some(command) = command {
            let _ = self.tx.send(command);
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
        for tab in state.tabs.iter() {
            constraints.push(Constraint::Length(tab.title.len() as u16));
        }

        let menu_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(constraints)
            .spacing(2)
            .split(menu_area);

        for (i, tab) in state.tabs.iter().enumerate() {
            let (title, style) = if i == self.selected_tab_index {
                (tab.title.to_uppercase(), Style::default().fg(Color::Green))
            } else {
                (tab.title.clone(), Style::default())
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
