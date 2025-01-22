use ratatui::{
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    widgets::Widget,
};

use crate::app::STATE;
use crate::message::ObjectId;
use crate::node_widget::NodeWidget;
use crate::state;

pub struct NodeList {
    top: usize,
    selected: Option<ObjectId>,
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
                .collect();

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

    pub fn up(&mut self) {
        self.move_selected(|selected| selected.saturating_sub(1));
    }

    pub fn down(&mut self) {
        self.move_selected(|selected| selected.saturating_add(1));
    }

    pub fn update(&mut self, area: Rect) {
        let nodes_visible = (area.height / NodeWidget::height()) as usize;
        STATE.with_borrow(|state| -> Option<()> {
            let nodes: Vec<&state::Node> = state
                .nodes
                .values()
                .filter(|node| (self.filter)(node))
                .collect();

            if self.top >= nodes.len() {
                self.top = nodes.len().saturating_sub(nodes_visible);
            }

            if let Some(selected) = self.selected {
                match nodes.iter().position(|node| node.id == selected) {
                    Some(selected_index) => {
                        if selected_index >= self.top + nodes_visible {
                            // Selected is above viewpoint, scroll up to it
                            self.top = selected_index.saturating_add(
                                selected_index - nodes_visible + 1,
                            );
                        } else if selected_index < self.top {
                            // Selected is below viewpoint, scroll down to it
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
