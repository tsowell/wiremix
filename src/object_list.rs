//! A Ratatui widget for an interactable list of PipeWire objects.

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{ListState, StatefulWidget, Widget},
};

use crossterm::event::{MouseButton, MouseEventKind};
use smallvec::smallvec;

use crate::app::{Action, MouseArea};
use crate::command::Command;
use crate::config::Config;
use crate::device_kind::DeviceKind;
use crate::device_widget::DeviceWidget;
use crate::dropdown_widget::DropdownWidget;
use crate::node_widget::NodeWidget;
use crate::object::ObjectId;
use crate::view::{self, ListKind, VolumeAdjustment};

/// ObjectList stores information for filtering and displaying a subset of
/// objects from a [`View`](`crate::view::View`).
///
/// Control operations pertaining to individual objects are handled here.
#[derive(Default)]
pub struct ObjectList {
    /// Index of the first object in viewport
    top: usize,
    /// ID of the currently selected object
    pub selected: Option<ObjectId>,
    /// Which set of objects to use from the View
    list_kind: ListKind,
    /// Default device type to use for defaults and node rendering
    device_kind: Option<DeviceKind>,
    /// Target dropdown state
    pub list_state: ListState,
    /// Targets
    pub targets: Vec<(view::Target, String)>,
}

impl ObjectList {
    pub fn new(list_kind: ListKind, device_kind: Option<DeviceKind>) -> Self {
        Self {
            top: 0,
            selected: None,
            list_kind,
            device_kind,
            ..Default::default()
        }
    }

    pub fn down(&mut self, view: &view::View) {
        if self.list_state.selected().is_some() {
            self.list_state.select_next();
        } else {
            let new_selected = { view.next_id(self.list_kind, self.selected) };
            if new_selected.is_some() {
                self.selected = new_selected;
            }
        }
    }

    pub fn up(&mut self, view: &view::View) {
        if self.list_state.selected().is_some() {
            self.list_state.select_previous();
        } else {
            let new_selected =
                { view.previous_id(self.list_kind, self.selected) };
            if new_selected.is_some() {
                self.selected = new_selected;
            }
        }
    }

    fn dropdown_open(&mut self, view: &view::View) {
        let targets = match self.list_kind {
            ListKind::Node(_) => self
                .selected
                .and_then(|object_id| view.node_targets(object_id)),
            ListKind::Device => self
                .selected
                .and_then(|object_id| view.device_targets(object_id)),
        };
        if let Some((targets, index)) = targets {
            if !targets.is_empty() {
                self.targets = targets;
                self.list_state.select(Some(index));
            }
        }
    }

    fn selected_target(&self) -> Option<&view::Target> {
        self.list_state
            .selected()
            .and_then(|index| self.targets.get(index))
            .map(|(target, _)| target)
    }

    pub fn dropdown_activate(&mut self, view: &view::View) -> Vec<Command> {
        // Just open the dropdown if it's not showing yet.
        if self.list_state.selected().is_none() {
            self.dropdown_open(view);
            return Vec::default();
        }

        let commands = self
            .selected
            .zip(self.selected_target())
            .map(|(object_id, &target)| view.set_target(object_id, target))
            .into_iter()
            .flatten()
            .collect();
        self.list_state.select(None);
        commands
    }

    pub fn dropdown_close(&mut self) {
        self.list_state.select(None);
    }

    pub fn set_target(
        &mut self,
        view: &view::View,
        target: view::Target,
    ) -> Vec<Command> {
        self.list_state.select(None);
        self.selected
            .map(|object_id| view.set_target(object_id, target))
            .into_iter()
            .flatten()
            .collect()
    }

    pub fn toggle_mute(&mut self, view: &view::View) -> Vec<Command> {
        if matches!(self.list_kind, ListKind::Device) {
            return Vec::new();
        }
        self.selected
            .and_then(|node_id| view.mute(node_id))
            .into_iter()
            .collect()
    }

    pub fn set_absolute_volume(
        &mut self,
        view: &view::View,
        volume: f32,
    ) -> Vec<Command> {
        if matches!(self.list_kind, ListKind::Device) {
            return Vec::new();
        }
        self.selected
            .and_then(|node_id| {
                view.volume(node_id, VolumeAdjustment::Absolute(volume))
            })
            .into_iter()
            .collect()
    }

    pub fn set_relative_volume(
        &mut self,
        view: &view::View,
        volume: f32,
    ) -> Vec<Command> {
        if matches!(self.list_kind, ListKind::Device) {
            return Vec::new();
        }
        self.selected
            .and_then(|node_id| {
                view.volume(node_id, VolumeAdjustment::Relative(volume))
            })
            .into_iter()
            .collect()
    }

    pub fn set_default(&mut self, view: &view::View) -> Vec<Command> {
        if matches!(self.list_kind, ListKind::Device) {
            return Vec::new();
        }
        self.selected
            .zip(self.device_kind)
            .and_then(|(node_id, device_kind)| {
                view.set_default(node_id, device_kind)
            })
            .into_iter()
            .collect()
    }

    /// Reconciles changes to objects, viewport, and selection.
    pub fn update(&mut self, area: Rect, view: &view::View) {
        let selected_index = self
            .selected
            .and_then(|selected| view.position(self.list_kind, selected));
        let objects_len = view.len(self.list_kind);

        let (_, list_area, _) = self.areas(&area);
        let full_height = match self.list_kind {
            ListKind::Node(_) => {
                NodeWidget::height().saturating_add(NodeWidget::spacing())
            }
            ListKind::Device => {
                DeviceWidget::height().saturating_add(DeviceWidget::spacing())
            }
        };
        let objects_visible = (list_area.height / full_height) as usize;

        // If objects were removed and the viewport is now below the visible
        // objects, move the viewport up so that the bottom of the object list
        // is visible.
        if self.top >= objects_len {
            self.top = objects_len.saturating_sub(objects_visible);
        }

        // Make sure the selected object is visible and adjust the viewport
        // if necessary.
        if self.selected.is_some() {
            match selected_index {
                Some(selected_index) => {
                    if selected_index
                        >= self.top.saturating_add(objects_visible)
                    {
                        // The selection is below the viewport. Reposition the
                        // viewport so that the selected item is at the bottom.
                        let objects_visible_except_last =
                            objects_visible.saturating_sub(1);
                        self.top = selected_index
                            .saturating_sub(objects_visible_except_last);
                    } else if selected_index < self.top {
                        // The selected item is above the viewport. Reposition
                        // so that it's the first visible item.
                        self.top = selected_index;
                    }
                }
                None => self.selected = None, // The selected object is gone!
            }
        }
    }

    fn areas(&self, area: &Rect) -> (Rect, Rect, Rect) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header_area
                Constraint::Min(0),    // list_area
                Constraint::Length(1), // footer_area
            ])
            .split(*area);

        (layout[0], layout[1], layout[2])
    }
}

pub struct ObjectListWidget<'a> {
    pub object_list: &'a mut ObjectList,
    pub view: &'a view::View,
    pub config: &'a Config,
}

struct ObjectListRenderContext<'a> {
    list_area: Rect,
    objects_layout: &'a [Rect],
    objects_visible: usize,
}

impl ObjectListWidget<'_> {
    fn render_node_list(
        &mut self,
        node_kind: view::NodeKind,
        context: ObjectListRenderContext,
        area: Rect,
        buf: &mut Buffer,
        mouse_areas: &mut Vec<MouseArea>,
    ) {
        let all_objects = self.view.full_nodes(node_kind);
        let objects = all_objects
            .iter()
            .skip(self.object_list.top)
            // Take one extra so we can render a partial node at the bottom of
            // the area.
            .take(context.objects_visible.saturating_add(1));

        let objects_and_areas: Vec<(&&view::Node, &Rect)> =
            objects.zip(context.objects_layout.iter()).collect();
        for (object, &object_area) in &objects_and_areas {
            let selected = self
                .object_list
                .selected
                .map(|id| id == object.id)
                .unwrap_or_default();
            NodeWidget::new(
                object,
                selected,
                self.object_list.device_kind,
                self.config,
            )
            .render(object_area, buf, mouse_areas);
        }

        // Show the target dropdown?
        if self.object_list.list_state.selected().is_some() {
            // Get the area for the selected object
            if let Some((_, object_area)) =
                objects_and_areas.iter().find(|(object, _)| {
                    self.object_list
                        .selected
                        .map(|id| id == object.id)
                        .unwrap_or_default()
                })
            {
                DropdownWidget::new(
                    self.object_list,
                    &NodeWidget::dropdown_area(
                        self.object_list,
                        &context.list_area,
                        object_area,
                    ),
                    self.config,
                )
                .render(area, buf, mouse_areas);
            }
        }
    }

    fn render_device_list(
        &mut self,
        context: ObjectListRenderContext,
        area: Rect,
        buf: &mut Buffer,
        mouse_areas: &mut Vec<MouseArea>,
    ) {
        let all_objects = self.view.full_devices();
        let objects = all_objects
            .iter()
            .skip(self.object_list.top)
            // Take one extra so we can render a partial node at the bottom of
            // the area.
            .take(context.objects_visible.saturating_add(1));

        let objects_and_areas: Vec<(&&view::Device, &Rect)> =
            objects.zip(context.objects_layout.iter()).collect();
        for (object, &object_area) in &objects_and_areas {
            let selected = self
                .object_list
                .selected
                .map(|id| id == object.id)
                .unwrap_or_default();
            DeviceWidget::new(object, selected, self.config).render(
                object_area,
                buf,
                mouse_areas,
            );
        }

        // Show the target dropdown?
        if self.object_list.list_state.selected().is_some() {
            // Get the area for the selected object
            if let Some((_, object_area)) =
                objects_and_areas.iter().find(|(object, _)| {
                    self.object_list
                        .selected
                        .map(|id| id == object.id)
                        .unwrap_or_default()
                })
            {
                DropdownWidget::new(
                    self.object_list,
                    &DeviceWidget::dropdown_area(
                        self.object_list,
                        &context.list_area,
                        object_area,
                    ),
                    self.config,
                )
                .render(area, buf, mouse_areas);
            }
        }
    }
}

impl StatefulWidget for &mut ObjectListWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        let (header_area, list_area, footer_area) =
            self.object_list.areas(&area);

        mouse_areas.push((
            header_area,
            smallvec![MouseEventKind::Down(MouseButton::Left)],
            smallvec![Action::MoveUp],
        ));

        mouse_areas.push((
            footer_area,
            smallvec![MouseEventKind::Down(MouseButton::Left)],
            smallvec![Action::MoveDown],
        ));

        mouse_areas.push((
            list_area,
            smallvec![MouseEventKind::ScrollUp],
            smallvec![Action::MoveUp],
        ));

        mouse_areas.push((
            list_area,
            smallvec![MouseEventKind::ScrollDown],
            smallvec![Action::MoveDown],
        ));

        let (spacing, height) = match self.object_list.list_kind {
            ListKind::Node(_) => (NodeWidget::spacing(), NodeWidget::height()),
            ListKind::Device => {
                (DeviceWidget::spacing(), DeviceWidget::height())
            }
        };

        let full_object_height = height.saturating_add(spacing);
        let objects_visible = (list_area.height / full_object_height) as usize;

        let len = self.view.len(self.object_list.list_kind);

        // Indicate we can scroll up if there are objects above the viewport.
        if self.object_list.top > 0 {
            Line::from(Span::styled(
                &self.config.char_set.list_more,
                self.config.theme.list_more,
            ))
            .alignment(Alignment::Center)
            .render(header_area, buf);
        }

        // Indicate we can scroll down if there are objects below the
        // viewport, with an exception for when the last row is partially
        // rendered but still has all the important parts rendered,
        // excluding margins, etc.
        let is_bottom_last =
            self.object_list.top.saturating_add(objects_visible)
                == len.saturating_sub(1);
        let is_bottom_enough =
            (list_area.height % full_object_height) >= height;
        if self.object_list.top.saturating_add(objects_visible) < len
            && !(is_bottom_last && is_bottom_enough)
        {
            Line::from(Span::styled(
                &self.config.char_set.list_more,
                self.config.theme.list_more,
            ))
            .alignment(Alignment::Center)
            .render(footer_area, buf);
        }

        let objects_layout = {
            let object_height = height;
            let mut constraints =
                vec![Constraint::Length(object_height); objects_visible];
            // A variable-length constraint for a partial last object
            constraints.push(Constraint::Max(object_height));
            let constraints = constraints;

            Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .spacing(spacing)
                .split(list_area)
        };

        match self.object_list.list_kind {
            ListKind::Node(node_kind) => {
                self.render_node_list(
                    node_kind,
                    ObjectListRenderContext {
                        list_area,
                        objects_layout: &objects_layout,
                        objects_visible,
                    },
                    area,
                    buf,
                    mouse_areas,
                );
            }
            ListKind::Device => {
                self.render_device_list(
                    ObjectListRenderContext {
                        list_area,
                        objects_layout: &objects_layout,
                        objects_visible,
                    },
                    area,
                    buf,
                    mouse_areas,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture_manager::CaptureManager;
    use crate::config;
    use crate::event::MonitorEvent;
    use crate::media_class::MediaClass;
    use crate::state::State;
    use crate::view::{ListKind, NodeKind, View};

    fn init() -> (State, CaptureManager) {
        let mut state = State::default();
        let mut capture_manager = CaptureManager::default();

        for i in 0..10 {
            let obj_id = ObjectId::from_raw_id(i);
            let events = vec![
                MonitorEvent::NodeDescription(
                    obj_id,
                    String::from("Test node"),
                ),
                MonitorEvent::NodeMediaClass(
                    obj_id,
                    MediaClass::from("Stream/Output/Audio"),
                ),
                MonitorEvent::NodeMediaName(obj_id, String::from("Media name")),
                MonitorEvent::NodeName(obj_id, String::from("Node name")),
                MonitorEvent::NodeObjectSerial(obj_id, i as i32),
                MonitorEvent::NodePeaks(obj_id, vec![0.0, 0.0], 512),
                MonitorEvent::NodePositions(obj_id, vec![0, 1]),
                MonitorEvent::NodeRate(obj_id, 44100),
                MonitorEvent::NodeVolumes(obj_id, vec![0.0, 0.0]),
                MonitorEvent::NodeMute(obj_id, false),
            ];
            for event in events {
                state.update(&mut capture_manager, event);
            }
        }

        (state, capture_manager)
    }

    #[test]
    fn object_list_up_overflow() {
        let (state, _) = init();
        let view = View::from(&state, &config::Names::default());

        let height = NodeWidget::height() + NodeWidget::spacing();
        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, height * 3 + 2);
        let mut object_list =
            ObjectList::new(ListKind::Node(NodeKind::All), None);
        // Select first object
        object_list.down(&view);
        assert_eq!(object_list.top, 0);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(0)));

        object_list.up(&view);
        object_list.update(rect, &view);
        assert_eq!(object_list.top, 0);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(0)));
    }

    #[test]
    fn object_list_down_overflow() {
        let (state, _) = init();
        let view = View::from(&state, &config::Names::default());

        let height = NodeWidget::height() + NodeWidget::spacing();
        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, height * 3 + 2);
        let mut object_list =
            ObjectList::new(ListKind::Node(NodeKind::All), None);
        // Select first object
        object_list.down(&view);
        assert_eq!(object_list.top, 0);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(0)));

        let nodes_len = view.nodes.len();

        for _ in 0..(nodes_len * 2) {
            object_list.down(&view);
        }

        object_list.update(rect, &view);
        assert_eq!(object_list.top, 7);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(9)));
    }

    #[test]
    fn object_list_remove_last_nodes() {
        let (mut state, mut capture_manager) = init();
        let view = View::from(&state, &config::Names::default());

        let height = NodeWidget::height() + NodeWidget::spacing();
        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, height * 3 + 2);
        let mut object_list =
            ObjectList::new(ListKind::Node(NodeKind::All), None);
        // Select first object
        object_list.down(&view);
        assert_eq!(object_list.top, 0);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(0)));

        let nodes_len = view.nodes.len();

        // Move to end of list
        for _ in 0..(nodes_len * 2) {
            object_list.down(&view);
        }
        object_list.update(rect, &view);
        assert_eq!(object_list.top, 7);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(9)));

        // Remove the visible nodes
        state.update(
            &mut capture_manager,
            MonitorEvent::Removed(ObjectId::from_raw_id(7)),
        );
        state.update(
            &mut capture_manager,
            MonitorEvent::Removed(ObjectId::from_raw_id(8)),
        );
        state.update(
            &mut capture_manager,
            MonitorEvent::Removed(ObjectId::from_raw_id(9)),
        );
        let view = View::from(&state, &config::Names::default());

        // Viewport is now below end of list

        object_list.update(rect, &view);
        assert_eq!(object_list.top, 4);
        assert_eq!(object_list.selected, None);
    }
}
