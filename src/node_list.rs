use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::app::STATE;
use crate::message::ObjectId;
use crate::meter;
use crate::named_constraints::with_named_constraints;
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
        let nodes_visible = (area.height / 5) as usize;
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
        let nodes_visible = (area.height / 5) as usize;
        STATE.with_borrow(|state| {
            let nodes = state
                .nodes
                .values()
                .filter(|node| (self.filter)(node))
                .skip(self.top)
                .take(nodes_visible);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(5); nodes_visible])
                .split(area);
            for (node, area) in nodes.zip(layout.iter()) {
                let selected =
                    self.selected.map(|id| id == node.id).unwrap_or_default();
                NodeWidget {
                    node: &node,
                    selected: selected,
                }
                .render(*area, buf);
            }
        });
    }
}

fn truncate(text: &str, len: usize) -> String {
    if text.len() <= len {
        return text.to_string();
    }

    let left = len.saturating_sub(3);

    let truncated = text
        .char_indices()
        .take_while(|(i, _)| *i < left)
        .map(|(_, c)| c)
        .collect::<String>();

    truncated + &".".repeat(std::cmp::min(len, 3))
}

fn is_default_for(node: &state::Node, which: &str) -> bool {
    STATE
        .with_borrow(|state| -> Option<bool> {
            let metadata = state.get_metadata_by_name("default")?;
            let json = metadata.properties.get(which)?;
            let obj = serde_json::from_str::<serde_json::Value>(json).ok()?;
            let default_name = obj["name"].as_str()?;
            let node_name = node.name.as_ref()?;
            Some(default_name == node_name)
        })
        .unwrap_or_default()
}

fn is_default(node: &state::Node) -> bool {
    is_default_for(node, "default.audio.sink")
        || is_default_for(node, "default.audio.source")
}

fn node_header_left(node: &state::Node) -> String {
    let default_string = if is_default(node) { "⯁ " } else { "" };
    let title = match (&node.description, &node.name, &node.media_name) {
        (_, Some(name), Some(media_name)) => format!("{name}: {media_name}"),
        (Some(description), _, _) => description.clone(),
        _ => String::new(),
    };
    format!("{}{}", default_string, title)
}

fn node_header_right(node: &state::Node) -> String {
    let Some(ref media_class) = node.media_class else {
        return Default::default();
    };
    match media_class.as_str() {
        "Audio/Sink" | "Audio/Source" => STATE
            .with_borrow(|state| -> Option<String> {
                let device_id = node.device_id?;
                let device = state.devices.get(&device_id)?;
                let route_index = device.route_index?;
                let route = device.routes.get(&route_index)?;
                Some(route.description.clone())
            })
            .unwrap_or_default(),
        "Stream/Output/Audio" => STATE
            .with_borrow(|state| -> Option<String> {
                let outputs = state.links.get(&node.id)?;
                for output in outputs {
                    let Some(output_node) = state.nodes.get(output) else {
                        continue;
                    };
                    let Some(ref media_class) = output_node.media_class else {
                        continue;
                    };
                    if media_class != "Audio/Sink" {
                        continue;
                    };
                    let description = output_node.description.as_ref()?;
                    return Some(description.to_owned());
                }

                None
            })
            .unwrap_or_default(),
        _ => Default::default(),
    }
}

struct NodeWidget<'a> {
    node: &'a state::Node,
    selected: bool,
}

impl<'a> Widget for NodeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = if self.selected {
            Style::default().bold()
        } else {
            Style::default()
        };

        let border_block = Block::default().borders(Borders::NONE);
        let mut header_area = Default::default();
        let mut bar_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut header_area)),
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut bar_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(border_block.inner(area))
            }
        );
        border_block.render(area, buf);

        let left = node_header_left(self.node);
        let right = node_header_right(self.node);

        let mut header_left = Default::default();
        let mut header_right = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), None),
                (Constraint::Min(0), Some(&mut header_left)),
                (Constraint::Length(1), None), // Padding
                (
                    Constraint::Length(right.len() as u16),
                    Some(&mut header_right)
                ),
                (Constraint::Length(1), None),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(header_area)
            }
        );

        Line::from(right)
            .style(style)
            .alignment(Alignment::Right)
            .render(header_right, buf);
        let left = truncate(&left, header_left.width as usize);
        Line::from(left).style(style).render(header_left, buf);

        let mut volume_area = Default::default();
        let mut meter_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), None),
                (Constraint::Fill(4), Some(&mut volume_area)),
                (Constraint::Fill(1), None),
                (Constraint::Fill(4), Some(&mut meter_area)),
                (Constraint::Fill(1), None),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(bar_area)
            }
        );

        let mut volume_label = Default::default();
        let mut volume_bar = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(4), Some(&mut volume_label)),
                (Constraint::Length(1), None), // Padding
                (Constraint::Min(0), Some(&mut volume_bar)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(volume_area)
            }
        );

        if let Some(volumes) = &self.node.volumes {
            if !volumes.is_empty() {
                let mean = volumes.iter().sum::<f32>() / volumes.len() as f32;
                let volume = mean.cbrt();
                let percent = (volume * 100.0) as u32;

                Line::from(format!("{}%", percent))
                    .style(style)
                    .alignment(Alignment::Right)
                    .render(volume_label, buf);

                let count = ((volume / 1.5) * volume_bar.width as f32) as usize;

                let filled = "━".repeat(count);
                let blank = "╌".repeat(volume_bar.width as usize - count);
                Line::from(vec![
                    Span::styled(filled, Style::default().fg(Color::Blue)),
                    Span::styled(blank, Style::default().fg(Color::Blue)),
                ])
                .render(volume_bar, buf);
            }
        }

        if let Some(positions) = &self.node.positions {
            match positions.len() {
                2 => meter::render_stereo(
                    meter_area,
                    buf,
                    self.node.peaks.as_ref().and_then(|peaks| {
                        (peaks.len() == 2).then_some((peaks[0], peaks[1]))
                    }),
                ),
                _ => meter::render_mono(
                    meter_area,
                    buf,
                    self.node.peaks.as_ref().and_then(|peaks| {
                        (!peaks.is_empty()).then_some(
                            peaks.iter().sum::<f32>() / peaks.len() as f32,
                        )
                    }),
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_test_equal() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_test_larger() {
        assert_eq!(truncate("hello", 6), "hello");
    }

    #[test]
    fn truncate_test_shorter() {
        assert_eq!(truncate("hello", 4), "h...");
    }

    #[test]
    fn truncate_test_too_short() {
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn truncate_test_much_too_short() {
        assert_eq!(truncate("hello", 2), "..");
    }

    #[test]
    fn truncate_test_empty() {
        assert_eq!(truncate("hello", 0), "");
    }
}
