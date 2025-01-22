use itertools::Itertools;

use ratatui::{
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    widgets::Widget,
};

use crate::app::STATE;
use crate::message::ObjectId;
use crate::node_widget::NodeWidget;
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

    fn move_selected(&mut self, movement: impl Fn(usize) -> usize) {
        STATE.with_borrow(|state| -> Option<()> {
            let nodes: Vec<&state::Node> = state
                .nodes
                .values()
                .filter(|node| (self.filter)(node))
                .sorted_by_key(|node| node.id)
                .collect();

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
        let nodes_visible = (area.height / NodeWidget::height()) as usize;
        STATE.with_borrow(|state| -> Option<()> {
            let nodes: Vec<&state::Node> = state
                .nodes
                .values()
                .filter(|node| (self.filter)(node))
                .sorted_by_key(|node| node.id)
                .collect();

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
                            self.top = selected_index.saturating_sub(
                                nodes_visible - 1,
                            );
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
}

impl Widget for &NodeList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let nodes_visible = (area.height / NodeWidget::height()) as usize;
        STATE.with_borrow(|state| {
            let nodes = state
                .nodes
                .values()
                .filter(|node| (self.filter)(node))
                .sorted_by_key(|node| node.id)
                .skip(self.top)
                .take(nodes_visible);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(NodeWidget::height());
                    nodes_visible
                ])
                .split(area);
            for (node, area) in nodes.zip(layout.iter()) {
                let selected =
                    self.selected.map(|id| id == node.id).unwrap_or_default();
                NodeWidget::new(&node, selected).render(*area, buf);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MonitorMessage;

    fn init() {
        STATE.with_borrow_mut(|state| {
            for i in 0..10 {
                let obj_id = ObjectId::from_raw_id(i);
                state.update(MonitorMessage::NodeDescription(
                    obj_id,
                    "Test node".to_string(),
                ));
            }
        });
    }

    #[test]
    fn node_list_up_overflow() {
        init();

        let rect = Rect::new(0, 0, 80, 18);
        let mut node_list = NodeList::new(Box::new(|_node| true));

        node_list.up();
        node_list.update(rect);
        assert_eq!(node_list.top, 0);
        assert_eq!(node_list.selected, Some(ObjectId::from_raw_id(0)));
    }

    #[test]
    fn node_list_down_overflow() {
        init();

        let rect = Rect::new(0, 0, 80, 18);
        let mut node_list = NodeList::new(Box::new(|_node| true));

        let nodes_len = STATE.with_borrow(|state| -> usize { state.nodes.len() });

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

        let rect = Rect::new(0, 0, 80, 18);
        let mut node_list = NodeList::new(Box::new(|_node| true));

        let nodes_len = STATE.with_borrow(|state| -> usize { state.nodes.len() });

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
