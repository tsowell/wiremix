use ratatui::{
    layout::Flex,
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, Padding, StatefulWidget,
        Widget,
    },
};

use crate::device_type::DeviceType;
use crate::meter;
use crate::named_constraints::with_named_constraints;
use crate::object_list::ObjectList;
use crate::truncate;
use crate::view;

fn is_default(node: &view::Node, device_type: Option<DeviceType>) -> bool {
    match device_type {
        Some(DeviceType::Sink) => node.is_default_sink,
        Some(DeviceType::Source) => node.is_default_source,
        None => false,
    }
}

fn node_title(node: &view::Node, device_type: Option<DeviceType>) -> String {
    (match (device_type, &node.title_source_sink) {
        (
            Some(DeviceType::Source | DeviceType::Sink),
            Some(title_source_sink),
        ) => title_source_sink,
        _ => &node.title,
    })
    .clone()
}

pub struct NodeWidget<'a> {
    node: &'a view::Node,
    selected: bool,
    device_type: Option<DeviceType>,
}

impl<'a> NodeWidget<'a> {
    pub fn new(
        node: &'a view::Node,
        selected: bool,
        device_type: Option<DeviceType>,
    ) -> Self {
        Self {
            node,
            selected,
            device_type,
        }
    }

    /// Height of a full node display.
    pub fn height() -> u16 {
        3
    }

    /// Spacing between nodes
    pub fn spacing() -> u16 {
        2
    }
}

impl Widget for NodeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (borders, padding) = if self.selected {
            (Borders::LEFT, Padding::ZERO)
        } else {
            (Borders::NONE, Padding::left(1))
        };

        let border_block = Block::default()
            .borders(borders)
            .padding(padding)
            .border_type(BorderType::Thick)
            .border_style(Style::new().fg(Color::Green));
        let mut header_area = Default::default();
        let mut bar_area = Default::default();
        let _layout = with_named_constraints!(
            [
                (Constraint::Length(1), Some(&mut header_area)),
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut bar_area)),
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

        let left = node_title(self.node, self.device_type);
        let right = match self.node.target {
            Some(view::Target::Default) => {
                format!("◇ {}", self.node.target_title)
            }
            _ => self.node.target_title.clone(),
        };

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
            .alignment(Alignment::Right)
            .render(header_right, buf);
        let default_string = if is_default(self.node, self.device_type) {
            "◇ "
        } else {
            "  "
        };
        let left =
            truncate::with_ellipses(&left, (header_left.width - 2) as usize);
        Line::from(vec![Span::from(default_string), Span::from(left)])
            .render(header_left, buf);

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
                (Constraint::Length(5), Some(&mut volume_label)),
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

        let volumes = &self.node.volumes;
        if !volumes.is_empty() {
            let mean = volumes.iter().sum::<f32>() / volumes.len() as f32;
            let volume = mean.cbrt();
            let percent = (volume * 100.0) as u32;

            Line::from(format!("{}%", percent))
                .alignment(Alignment::Right)
                .render(volume_label, buf);

            let count = ((volume.clamp(0.0, 1.5) / 1.5)
                * volume_bar.width as f32) as usize;

            let filled = "━".repeat(count);
            let blank = "╌".repeat(volume_bar.width as usize - count);
            Line::from(vec![
                Span::styled(filled, Style::default().fg(Color::Blue)),
                Span::styled(blank, Style::default().fg(Color::DarkGray)),
            ])
            .render(volume_bar, buf);
        }
        if self.node.mute {
            Line::from("muted").render(volume_label, buf);
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

pub struct NodePopupWidget<'a> {
    object_list: &'a mut ObjectList,
    list_area: &'a Rect,
    parent_area: &'a Rect,
}

impl<'a> NodePopupWidget<'a> {
    pub fn new(
        object_list: &'a mut ObjectList,
        list_area: &'a Rect,
        parent_area: &'a Rect,
    ) -> Self {
        Self {
            object_list,
            list_area,
            parent_area,
        }
    }
}

impl Widget for NodePopupWidget<'_> {
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
