//! A Ratatui widget for an interactable list of PipeWire objects.

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{ListState, StatefulWidget, Widget},
};

use crossterm::event::{MouseButton, MouseEventKind};
use smallvec::smallvec;

use crate::app::{Action, MouseArea};
use crate::config::Config;
use crate::device_kind::DeviceKind;
use crate::device_widget::DeviceWidget;
use crate::dropdown_widget::DropdownWidget;
use crate::node_widget::NodeWidget;
use crate::view::{self, ListKind, VolumeAdjustment};
use crate::wirehose::ObjectId;

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
    pub dropdown_state: ListState,
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
        if self.dropdown_state.selected().is_some() {
            self.dropdown_state.select_next();
        } else {
            let new_selected = view.next_id(self.list_kind, self.selected);
            if new_selected.is_some() {
                self.select(new_selected);
            }
        }
    }

    pub fn up(&mut self, view: &view::View) {
        if self.dropdown_state.selected().is_some() {
            self.dropdown_state.select_previous();
        } else {
            let new_selected = view.previous_id(self.list_kind, self.selected);
            if new_selected.is_some() {
                self.select(new_selected);
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
                self.dropdown_state.select(Some(index));
            }
        }
    }

    fn selected_target(&self) -> Option<&view::Target> {
        self.dropdown_state
            .selected()
            .and_then(|index| self.targets.get(index))
            .map(|(target, _)| target)
    }

    pub fn dropdown_activate(&mut self, view: &view::View) {
        // Just open the dropdown if it's not showing yet.
        if self.dropdown_state.selected().is_none() {
            self.dropdown_open(view);
            return;
        }

        if let (Some(object_id), Some(&target)) =
            (self.selected, self.selected_target())
        {
            view.set_target(object_id, target);
        };

        self.dropdown_state.select(None);
    }

    pub fn dropdown_close(&mut self) {
        self.dropdown_state.select(None);
    }

    pub fn set_target(&mut self, view: &view::View, target: view::Target) {
        self.dropdown_state.select(None);
        if let Some(object_id) = self.selected {
            view.set_target(object_id, target);
        };
    }

    pub fn toggle_mute(&mut self, view: &view::View) {
        if matches!(self.list_kind, ListKind::Device) {
            return;
        }
        if let Some(node_id) = self.selected {
            view.mute(node_id);
        }
    }

    pub fn set_absolute_volume(
        &mut self,
        view: &view::View,
        volume: f32,
        max: Option<f32>,
    ) -> bool {
        if matches!(self.list_kind, ListKind::Device) {
            return false;
        }
        if let Some(node_id) = self.selected {
            return view.volume(
                node_id,
                VolumeAdjustment::Absolute(volume),
                max,
            );
        }
        false
    }

    pub fn set_relative_volume(
        &mut self,
        view: &view::View,
        volume: f32,
        max: Option<f32>,
    ) -> bool {
        if matches!(self.list_kind, ListKind::Device) {
            return false;
        }
        if let Some(node_id) = self.selected {
            return view.volume(
                node_id,
                VolumeAdjustment::Relative(volume),
                max,
            );
        }
        false
    }

    pub fn set_absolute_balance(
        &mut self,
        view: &view::View,
        balance: f32,
        max: Option<f32>,
    ) -> bool {
        if matches!(self.list_kind, ListKind::Device) {
            return false;
        }
        if let Some(node_id) = self.selected {
            return view.volume(
                node_id,
                VolumeAdjustment::AbsoluteBalance(balance),
                max,
            );
        }
        false
    }

    pub fn set_relative_balance(
        &mut self,
        view: &view::View,
        balance: f32,
        max: Option<f32>,
    ) -> bool {
        if matches!(self.list_kind, ListKind::Device) {
            return false;
        }
        if let Some(node_id) = self.selected {
            return view.volume(
                node_id,
                VolumeAdjustment::RelativeBalance(balance),
                max,
            );
        }
        false
    }

    pub fn set_default(&mut self, view: &view::View) {
        if matches!(self.list_kind, ListKind::Device) {
            return;
        }
        if let (Some(node_id), Some(device_kind)) =
            (self.selected, self.device_kind)
        {
            view.set_default(node_id, device_kind);
        }
    }

    fn selected_index(&self, view: &view::View) -> Option<usize> {
        self.selected
            .and_then(|selected| view.position(self.list_kind, selected))
    }

    fn select(&mut self, object_id: Option<ObjectId>) {
        self.selected = object_id;
        // Close the dropdown in case it is open for the previously-selected
        // object. This can happen when the object is removed from PipeWire
        // while the dropdown is open.
        self.dropdown_close();
    }

    /// Reconciles changes to objects, viewport, and selection.
    pub fn update(&mut self, area: Rect, view: &view::View) {
        let selected_index = self.selected_index(view).or_else(|| {
            // There's nothing selected! Select the first item and try again.
            self.select(view.next_id(self.list_kind, None));
            self.selected_index(view)
        });

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
                None => self.select(None), // The selected object is gone!
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

pub struct ObjectListWidget<'a, 'b> {
    pub object_list: &'a mut ObjectList,
    pub view: &'a view::View<'b>,
    pub config: &'a Config,
}

struct ObjectListRenderContext<'a> {
    list_area: Rect,
    objects_layout: &'a [Rect],
    objects_visible: usize,
}

impl ObjectListWidget<'_, '_> {
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
                .map(|id| id == object.object_id)
                .unwrap_or_default();
            NodeWidget::new(
                self.config,
                self.object_list.device_kind,
                object,
                selected,
            )
            .render(object_area, buf, mouse_areas);
        }

        // Show the target dropdown?
        if self.object_list.dropdown_state.selected().is_some() {
            // Get the area for the selected object
            if let Some((_, object_area)) =
                objects_and_areas.iter().find(|(object, _)| {
                    self.object_list
                        .selected
                        .map(|id| id == object.object_id)
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
                .map(|id| id == object.object_id)
                .unwrap_or_default();
            DeviceWidget::new(object, selected, self.config).render(
                object_area,
                buf,
                mouse_areas,
            );
        }

        // Show the target dropdown?
        if self.object_list.dropdown_state.selected().is_some() {
            // Get the area for the selected object
            if let Some((_, object_area)) =
                objects_and_areas.iter().find(|(object, _)| {
                    self.object_list
                        .selected
                        .map(|id| id == object.object_id)
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

impl StatefulWidget for &mut ObjectListWidget<'_, '_> {
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
    use crate::config;
    use crate::mock;
    use crate::view::{ListKind, NodeKind, View};
    use crate::wirehose::{state::State, PropertyStore, StateEvent};

    fn init() -> (State, mock::WirehoseHandle) {
        let mut state = State::default();
        let wirehose = mock::WirehoseHandle::default();

        for i in 0..10 {
            let object_id = ObjectId::from_raw_id(i);
            let mut props = PropertyStore::default();
            props.set_node_description(String::from("Test node"));
            props.set_media_class(String::from("Stream/Output/Audio"));
            props.set_media_name(String::from("Media name"));
            props.set_node_name(String::from("Node name"));
            props.set_object_serial(i as u64);
            let props = props;

            let events = vec![
                StateEvent::NodeProperties { object_id, props },
                StateEvent::NodePeaks {
                    object_id,
                    peaks: vec![0.0, 0.0],
                    samples: 512,
                },
                StateEvent::NodePositions {
                    object_id,
                    positions: vec![0, 1],
                },
                StateEvent::NodeRate {
                    object_id,
                    rate: 44100,
                },
                StateEvent::NodeVolumes {
                    object_id,
                    volumes: vec![0.0, 0.0],
                },
                StateEvent::NodeMute {
                    object_id,
                    mute: false,
                },
            ];
            for event in events {
                state.update(&wirehose, event);
            }
        }

        (state, wirehose)
    }

    #[test]
    fn object_list_up_overflow() {
        let (state, wirehose) = init();
        let view = View::from(&wirehose, &state, &config::Names::default());

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
        let (state, wirehose) = init();
        let view = View::from(&wirehose, &state, &config::Names::default());

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
}
