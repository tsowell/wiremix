use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListState, StatefulWidget, Widget},
};

use crate::device_type::DeviceType;
use crate::named_constraints::with_named_constraints;
use crate::node_widget::NodeWidget;
use crate::object::ObjectId;
use crate::view::{self, ListType};

/// ObjectList stores information for filtering and displaying a subset of
/// objects from the global STATE.
#[derive(Default)]
pub struct ObjectList {
    /// Index of the first node in viewport
    top: usize,
    /// ID of the currently selected node
    pub selected: Option<ObjectId>,
    /// Which set of nodes to use from the View
    pub list_type: ListType,
    /// Default device type to use
    pub device_type: Option<DeviceType>,
    /// Target popup state
    pub list_state: ListState,
    /// Targets
    pub targets: Vec<(view::Target, String)>,
}

impl ObjectList {
    pub fn new(list_type: ListType, device_type: Option<DeviceType>) -> Self {
        Self {
            top: 0,
            selected: None,
            list_type,
            device_type,
            ..Default::default()
        }
    }

    pub fn selected_target(&self) -> Option<&view::Target> {
        self.list_state
            .selected()
            .and_then(|index| self.targets.get(index))
            .map(|(target, _)| target)
    }

    /// Reconciles changes to nodes, viewport, and selection.
    pub fn update(
        &mut self,
        area: Rect,
        selected_index: Option<usize>,
        nodes_len: usize,
    ) {
        let (_, list_area, _) = self.areas(&area);
        let nodes_visible = (list_area.height / NodeWidget::height()) as usize;

        // If nodes were removed and the viewport is now below the visible
        // nodes, move the viewport up so that the bottom of the node list
        // is visible.
        if self.top >= nodes_len {
            self.top = nodes_len.saturating_sub(nodes_visible);
        }

        // Make sure the selected node is visible and adjust the viewport
        // if necessary.
        if self.selected.is_some() {
            match selected_index {
                Some(selected_index) => {
                    if selected_index >= self.top + nodes_visible {
                        self.top =
                            selected_index.saturating_sub(nodes_visible - 1);
                    } else if selected_index < self.top {
                        self.top = selected_index;
                    }
                }
                None => self.selected = None, // The selected node is gone!
            }
        }
    }

    fn areas(&self, area: &Rect) -> (Rect, Rect, Rect) {
        let mut header_area = Default::default();
        let mut list_area = Default::default();
        let mut footer_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), Some(&mut header_area)),
                (Constraint::Min(0), Some(&mut list_area)),
                (Constraint::Length(1), Some(&mut footer_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(*area)
            }
        );

        (header_area, list_area, footer_area)
    }
}

pub struct ObjectListWidget<'a> {
    pub object_list: &'a mut ObjectList,
    pub view: &'a view::View,
}

impl<'a> ObjectListWidget<'a> {
    fn render_node_list(
        &mut self,
        node_type: view::NodeType,
        list_area: Rect,
        nodes_layout: &[Rect],
        nodes_visible: usize,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let all_nodes = self.view.full_nodes(node_type);
        let nodes = all_nodes
            .iter()
            .skip(self.object_list.top)
            // Take one extra so we can render a partial node at the bottom of
            // the area.
            .take(nodes_visible + 1);

        let nodes_and_areas: Vec<(&&view::Node, &Rect)> =
            nodes.zip(nodes_layout.iter()).collect();
        for (node, &node_area) in &nodes_and_areas {
            let selected = self
                .object_list
                .selected
                .map(|id| id == node.id)
                .unwrap_or_default();
            NodeWidget::new(node, selected, self.object_list.device_type)
                .render(node_area, buf);
        }

        // Show the target popup?
        if self.object_list.list_state.selected().is_some() {
            // Get the area for the selected node
            if let Some((_, node_area)) =
                nodes_and_areas.iter().find(|(node, _)| {
                    self.object_list
                        .selected
                        .map(|id| id == node.id)
                        .unwrap_or_default()
                })
            {
                PopupWidget {
                    object_list: self.object_list,
                    list_area: &list_area,
                    parent_area: &area,
                }
                .render(**node_area, buf);
            }
        }
    }
}

impl Widget for &mut ObjectListWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (header_area, list_area, footer_area) =
            self.object_list.areas(&area);

        let spacing = 2;
        let node_height_with_spacing = NodeWidget::height() + spacing;
        let nodes_visible =
            (list_area.height / node_height_with_spacing) as usize;

        let len = self.view.len(self.object_list.list_type);

        // Indicate we can scroll up if there are nodes above the viewport.
        if self.object_list.top > 0 {
            Line::from(Span::styled(
                "•••",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Center)
            .render(header_area, buf);
        }

        // Indicate we can scroll down if there are nodes below the
        // viewport, with an exception for when the last row is partially
        // rendered but still has all the important parts rendered,
        // excluding margins, etc.
        let is_bottom_last =
            self.object_list.top + nodes_visible == len.saturating_sub(1);
        let is_bottom_enough = (list_area.height % node_height_with_spacing)
            >= NodeWidget::important_height();
        if self.object_list.top + nodes_visible < len
            && !(is_bottom_last && is_bottom_enough)
        {
            Line::from(Span::styled(
                "•••",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Center)
            .render(footer_area, buf);
        }

        let nodes_layout = {
            let node_height = NodeWidget::height();
            let mut constraints =
                vec![Constraint::Length(node_height); nodes_visible];
            // A variable-length constraint for a partial last node
            constraints.push(Constraint::Max(node_height));

            Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .spacing(spacing)
                .split(list_area)
        };

        match self.object_list.list_type {
            ListType::Node(node_type) => {
                self.render_node_list(
                    node_type,
                    list_area,
                    &nodes_layout,
                    nodes_visible,
                    area,
                    buf,
                );
            }
            ListType::Device => todo!(),
        }
    }
}

struct PopupWidget<'a> {
    object_list: &'a mut ObjectList,
    list_area: &'a Rect,
    parent_area: &'a Rect,
}

impl Widget for PopupWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let targets: Vec<_> = self
            .object_list
            .targets
            .iter()
            .map(|(_, title)| title.clone())
            .collect();
        let max_target_length =
            targets.iter().map(|s| s.len()).max().unwrap_or(0);

        let popup_area = Rect::new(
            self.list_area.right() - max_target_length as u16 - 2,
            area.top() - 1,
            max_target_length as u16 + 2,
            std::cmp::min(7, targets.len() as u16 + 2),
        )
        .clamp(*self.parent_area);

        Clear.render(popup_area, buf);

        let list = List::new(targets)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::REVERSED),
            );

        StatefulWidget::render(
            &list,
            popup_area,
            buf,
            &mut self.object_list.list_state,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::MonitorEvent;

    fn init() {
        STATE.with_borrow_mut(|state| {
            for i in 0..10 {
                let obj_id = ObjectId::from_raw_id(i);
                state.update(MonitorEvent::NodeDescription(
                    obj_id,
                    "Test node".to_string(),
                ));
            }
        });
    }

    #[test]
    fn object_list_up_overflow() {
        init();

        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, NodeWidget::height() * 3 + 2);
        let mut object_list = ObjectList::new(Box::new(|_node| true));

        object_list.up();
        object_list.update(rect);
        assert_eq!(object_list.top, 0);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(0)));
    }

    #[test]
    fn object_list_down_overflow() {
        init();

        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, NodeWidget::height() * 3 + 2);
        let mut object_list = ObjectList::new(Box::new(|_node| true));

        let nodes_len =
            STATE.with_borrow(|state| -> usize { state.nodes.len() });

        for _ in 0..(nodes_len * 2) {
            object_list.down();
        }

        object_list.update(rect);
        assert_eq!(object_list.top, 7);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(9)));
    }

    #[test]
    fn object_list_remove_last_nodes() {
        init();

        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, NodeWidget::height() * 3 + 2);
        let mut object_list = ObjectList::new(Box::new(|_node| true));

        let nodes_len =
            STATE.with_borrow(|state| -> usize { state.nodes.len() });

        // Move to end of list
        for _ in 0..(nodes_len * 2) {
            object_list.down();
        }
        object_list.update(rect);
        assert_eq!(object_list.top, 7);
        assert_eq!(object_list.selected, Some(ObjectId::from_raw_id(9)));

        // Remove the visible nodes
        STATE.with_borrow_mut(|state| {
            state.nodes.remove(&ObjectId::from_raw_id(7));
            state.nodes.remove(&ObjectId::from_raw_id(8));
            state.nodes.remove(&ObjectId::from_raw_id(9));
        });
        // Viewport is now below end of list

        object_list.update(rect);
        assert_eq!(object_list.top, 4);
        assert_eq!(object_list.selected, None);
    }
}
