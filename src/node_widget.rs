use ratatui::{
    layout::Flex,
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use crate::app::STATE;
use crate::meter;
use crate::named_constraints::with_named_constraints;
use crate::state;
use crate::truncate;

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
                let outputs = state.outputs(node.id);
                for output in outputs {
                    let Some(output_node) = state.nodes.get(&output) else {
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
        "Stream/Input/Audio" => STATE
            .with_borrow(|state| -> Option<String> {
                let inputs = state.inputs(node.id);
                for input in inputs {
                    let Some(input_node) = state.nodes.get(&input) else {
                        continue;
                    };
                    let Some(ref media_class) = input_node.media_class else {
                        continue;
                    };
                    if media_class != "Audio/Source" {
                        continue;
                    };
                    let description = input_node.description.as_ref()?;
                    return Some(description.to_owned());
                }

                None
            })
            .unwrap_or_default(),
        _ => Default::default(),
    }
}

pub struct NodeWidget<'a> {
    node: &'a state::Node,
    selected: bool,
}

impl<'a> NodeWidget<'a> {
    pub fn new(node: &'a state::Node, selected: bool) -> Self {
        Self { node, selected }
    }

    /// Height of a full node display.
    pub fn height() -> u16 {
        5
    }

    /// Height of the important parts (excluding blank margin lines at bottom).
    pub fn important_height() -> u16 {
        3
    }
}

impl<'a> Widget for NodeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = if self.selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let border_block = Block::default().borders(Borders::NONE);
        let mut header_area = Default::default();
        let mut bar_area = Default::default();
        let _layout = with_named_constraints!(
            [
                (Constraint::Length(1), Some(&mut header_area)),
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut bar_area)),
                (Constraint::Length(1), None),
                (Constraint::Length(1), None),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .flex(Flex::Legacy)
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
        let left = truncate::with_ellipses(&left, header_left.width as usize);
        Line::from(left).style(style).render(header_left, buf);

        let mut volume_area = Default::default();
        let mut meter_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(2), None),
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

        match self.node.peaks.as_deref() {
            Some([left, right]) => {
                meter::render_stereo(meter_area, buf, Some((*left, *right)))
            }
            Some(peaks @ [..]) => meter::render_mono(
                meter_area,
                buf,
                (!peaks.is_empty())
                    .then_some(peaks.iter().sum::<f32>() / peaks.len() as f32),
            ),
            _ => match self
                .node
                .positions
                .as_ref()
                .map(|positions| positions.len())
            {
                Some(2) => meter::render_stereo(meter_area, buf, None),
                _ => meter::render_mono(meter_area, buf, None),
            },
        }
    }
}
