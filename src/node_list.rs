use itertools::Itertools;

use serde_json::json;

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::app::STATE;
use crate::command::Command;
use crate::named_constraints::with_named_constraints;
use crate::node_widget::NodeWidget;
use crate::object::ObjectId;
use crate::state;

/// NodeList stores information for filtering and displaying a subset of Nodes
/// from the global STATE.
pub struct NodeList {
    /// Index of the first node in viewport
    top: usize,
    /// ID of the currently selected node
    selected: Option<ObjectId>,
    /// Predicate by which to filter the global nodes
    filter: Box<dyn Fn(&state::Node) -> bool>,
}

impl NodeList {
    pub fn new(filter: Box<dyn Fn(&state::Node) -> bool>) -> Self {
        Self {
            top: 0,
            selected: None,
            filter,
        }
    }

    pub fn set_default(&self) -> Option<Command> {
        let node_id = self.selected?;

        STATE.with_borrow(|state| {
            let node = state.nodes.get(&node_id)?;
            let name = node.name.clone()?;
            let key = match node.media_class.as_ref()?.as_str() {
                "Audio/Sink" => Some("default.configured.audio.sink"),
                "Audio/Source" => Some("default.configured.audio.source"),
                _ => None,
            }?;
            let metadata_id = state.metadatas_by_name.get("default")?;

            Some(Command::MetadataSetProperty(
                *metadata_id,
                0,
                String::from(key),
                Some(String::from("Spa:String:JSON")),
                Some(json!({ "name": name }).to_string()),
            ))
        })
    }

    pub fn volume(&self, change: impl FnOnce(f32) -> f32) -> Option<Command> {
        let node_id = self.selected?;

        STATE.with_borrow(|state| {
            let node = state.nodes.get(&node_id)?;
            let mut volumes = node.volumes.clone()?;
            if volumes.is_empty() {
                return None;
            }

            let avg = volumes.iter().sum::<f32>() / volumes.len() as f32;
            volumes.fill(change(avg.cbrt()).max(0.0).powi(3));

            if let Some(device_id) = node.device_id {
                let device = state.devices.get(&device_id)?;
                let route_index = device.route_index?;
                let route_device = device.route_device?;

                Some(Command::DeviceVolumes(
                    device_id,
                    route_index,
                    route_device,
                    volumes,
                ))
            } else {
                Some(Command::NodeVolumes(node_id, volumes))
            }
        })
    }

    fn filtered_nodes<'a>(
        &self,
        state: &'a state::State,
    ) -> Vec<&'a state::Node> {
        state
            .nodes
            .values()
            .filter(|node| (self.filter)(node))
            .sorted_by_key(|node| node.id)
            .collect()
    }

    fn move_selected(&mut self, movement: impl Fn(usize) -> usize) {
        STATE.with_borrow(|state| -> Option<()> {
            let nodes = self.filtered_nodes(state);

            // TODO cache the selected index
            let new_selected_index = match self.selected {
                None => 0,
                Some(selected) => {
                    movement(nodes.iter().position(|node| node.id == selected)?)
                }
            };

            if let Some(new_node) = nodes.get(new_selected_index) {
                self.selected = Some(new_node.id);
            }

            Some(())
        });
    }

    /// Selects the previous node.
    pub fn up(&mut self) {
        self.move_selected(|selected| selected.saturating_sub(1));
    }

    /// Selects the next node.
    pub fn down(&mut self) {
        self.move_selected(|selected| selected.saturating_add(1));
    }

    /// Reconciles changes to nodes, viewport, and selection.
    pub fn update(&mut self, area: Rect) {
        let (_, list_area, _) = self.areas(&area);
        let nodes_visible = (list_area.height / NodeWidget::height()) as usize;
        STATE.with_borrow(|state| -> Option<()> {
            let nodes = self.filtered_nodes(state);

            // If nodes were removed and the viewport is now below the visible
            // nodes, move the viewport up so that the bottom of the node list
            // is visible.
            if self.top >= nodes.len() {
                self.top = nodes.len().saturating_sub(nodes_visible);
            }

            // Make sure the selected node is visible and adjust the viewport
            // if necessary.
            if let Some(selected) = self.selected {
                match nodes.iter().position(|node| node.id == selected) {
                    Some(selected_index) => {
                        if selected_index >= self.top + nodes_visible {
                            self.top = selected_index
                                .saturating_sub(nodes_visible - 1);
                        } else if selected_index < self.top {
                            self.top = selected_index;
                        }
                    }
                    None => self.selected = None, // The selected node is gone!
                }
            }

            Some(())
        });
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

impl Widget for &NodeList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        STATE.with_borrow(|state| {
            let (header_area, list_area, footer_area) = self.areas(&area);

            let node_height = NodeWidget::height();
            let nodes_visible = (list_area.height / node_height) as usize;

            let all_nodes = self.filtered_nodes(state);
            let nodes = all_nodes
                .iter()
                .skip(self.top)
                // Take one extra so we can render a partial node at the bottom
                // of the area.
                .take(nodes_visible + 1);

            // Indicate we can scroll up if there are nodes above the viewport.
            if self.top > 0 {
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
                self.top + nodes_visible == all_nodes.len().saturating_sub(1);
            let is_bottom_enough = (list_area.height % node_height)
                >= NodeWidget::important_height();
            if self.top + nodes_visible < all_nodes.len()
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
                let mut constraints =
                    vec![Constraint::Length(node_height); nodes_visible];
                // A variable-length constraint for a partial last node
                constraints.push(Constraint::Max(node_height));

                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(list_area)
            };
            for (node, area) in nodes.zip(nodes_layout.iter()) {
                let selected =
                    self.selected.map(|id| id == node.id).unwrap_or_default();
                NodeWidget::new(node, selected).render(*area, buf);
            }
        });
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
    fn node_list_up_overflow() {
        init();

        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, NodeWidget::height() * 3 + 2);
        let mut node_list = NodeList::new(Box::new(|_node| true));

        node_list.up();
        node_list.update(rect);
        assert_eq!(node_list.top, 0);
        assert_eq!(node_list.selected, Some(ObjectId::from_raw_id(0)));
    }

    #[test]
    fn node_list_down_overflow() {
        init();

        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, NodeWidget::height() * 3 + 2);
        let mut node_list = NodeList::new(Box::new(|_node| true));

        let nodes_len =
            STATE.with_borrow(|state| -> usize { state.nodes.len() });

        for _ in 0..(nodes_len * 2) {
            node_list.down();
        }

        node_list.update(rect);
        assert_eq!(node_list.top, 7);
        assert_eq!(node_list.selected, Some(ObjectId::from_raw_id(9)));
    }

    #[test]
    fn node_list_remove_last_nodes() {
        init();

        // + 2 for header and footer
        let rect = Rect::new(0, 0, 80, NodeWidget::height() * 3 + 2);
        let mut node_list = NodeList::new(Box::new(|_node| true));

        let nodes_len =
            STATE.with_borrow(|state| -> usize { state.nodes.len() });

        // Move to end of list
        for _ in 0..(nodes_len * 2) {
            node_list.down();
        }
        node_list.update(rect);
        assert_eq!(node_list.top, 7);
        assert_eq!(node_list.selected, Some(ObjectId::from_raw_id(9)));

        // Remove the visible nodes
        STATE.with_borrow_mut(|state| {
            state.nodes.remove(&ObjectId::from_raw_id(7));
            state.nodes.remove(&ObjectId::from_raw_id(8));
            state.nodes.remove(&ObjectId::from_raw_id(9));
        });
        // Viewport is now below end of list

        node_list.update(rect);
        assert_eq!(node_list.top, 4);
        assert_eq!(node_list.selected, None);
    }
}
