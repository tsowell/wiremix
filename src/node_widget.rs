//! A Ratatui widget representing a single PipeWire node in an object list.

use ratatui::{
    layout::Flex,
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

use crossterm::event::{MouseButton, MouseEventKind};

use crate::app::{Action, MouseArea};
use crate::device_type::DeviceType;
use crate::meter;
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

    /// Area for the target popup
    pub fn popup_area(
        object_list: &ObjectList,
        list_area: &Rect,
        object_area: &Rect,
    ) -> Rect {
        // Number of items to show at once
        let max_visible_items = 5;

        let max_target_length = object_list
            .targets
            .iter()
            .map(|(_, title)| title.len())
            .max()
            .unwrap_or(0);

        // Add 2 for vertical borders and 2 for highlight symbol
        let width = max_target_length.saturating_add(4) as u16;
        let height = std::cmp::min(max_visible_items, object_list.targets.len())
            .saturating_add(2) as u16; // Plus 2 for horizontal borders

        // Align to the right of the list area
        let x = list_area.right().saturating_sub(width);
        // Subtract 1 for the top border
        let y = object_area.top().saturating_sub(1);

        Rect::new(x, y, width, height)
    }
}

impl StatefulWidget for NodeWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        mouse_areas.push((
            area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::SelectObject(self.node.id)],
        ));

        mouse_areas.push((
            area,
            vec![MouseEventKind::Down(MouseButton::Right)],
            vec![Action::SelectObject(self.node.id), Action::SetDefault],
        ));

        mouse_areas.push((
            area,
            vec![MouseEventKind::ScrollLeft],
            vec![
                Action::SelectObject(self.node.id),
                Action::SetRelativeVolume(-0.01),
            ],
        ));

        mouse_areas.push((
            area,
            vec![MouseEventKind::ScrollRight],
            vec![
                Action::SelectObject(self.node.id),
                Action::SetRelativeVolume(0.01),
            ],
        ));

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1), // selected_area
                Constraint::Min(0),    // node_area
            ])
            .split(area);
        let selected_area = layout[0];
        let node_area = layout[1];

        if self.selected {
            // Render and indication that this is the selected node.
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(selected_area);

            let style = Style::default().fg(Color::LightCyan);

            Line::from(Span::styled("░", style)).render(rows[0], buf);
            Line::from(Span::styled("▒", style)).render(rows[1], buf);
            Line::from(Span::styled("░", style)).render(rows[2], buf);
        }

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // header_area
                Constraint::Length(1), // bar_area
            ])
            .spacing(1)
            .flex(Flex::Legacy)
            .split(node_area);
        let header_area = layout[0];
        let bar_area = layout[1];

        let node_title = node_title(self.node, self.device_type);
        let target_title = match self.node.target {
            Some(view::Target::Default) => {
                format!("◇ {}", self.node.target_title)
            }
            _ => self.node.target_title.clone(),
        };

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),                            // header_left
                Constraint::Length(target_title.len() as u16), // header_right
            ])
            .horizontal_margin(1)
            .spacing(1)
            .split(header_area);
        let header_left = layout[0];
        let header_right = layout[1];

        Line::from(target_title)
            .alignment(Alignment::Right)
            .render(header_right, buf);
        mouse_areas.push((
            header_right,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::SelectObject(self.node.id), Action::OpenPopup],
        ));

        let default_string = if is_default(self.node, self.device_type) {
            "◇ "
        } else {
            "  "
        };
        let node_title = truncate::with_ellipses(
            &node_title,
            (header_left.width.saturating_sub(2)) as usize,
        );
        Line::from(vec![Span::from(default_string), Span::from(node_title)])
            .render(header_left, buf);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2), // _padding
                Constraint::Fill(4),   // volume_area
                Constraint::Fill(1),   // _padding
                Constraint::Fill(4),   // meter_area
                Constraint::Fill(1),   // _padding
            ])
            .split(bar_area);
        // index 0 is _padding
        let volume_area = layout[1];
        // index 2 is _padding
        let meter_area = layout[3];
        // index 4 is _padding

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(5), // volume_label
                Constraint::Min(0),    // volume_bar
            ])
            .spacing(1)
            .split(volume_area);
        let volume_label = layout[0];
        let volume_bar = layout[1];

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
            let blank =
                "╌".repeat((volume_bar.width as usize).saturating_sub(count));
            Line::from(vec![
                Span::styled(filled, Style::default().fg(Color::Blue)),
                Span::styled(blank, Style::default().fg(Color::DarkGray)),
            ])
            .render(volume_bar, buf);
        }
        if self.node.mute {
            Line::from("muted").render(volume_label, buf);
        }

        mouse_areas.push((
            volume_label,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::SelectObject(self.node.id), Action::ToggleMute],
        ));

        // Add mouse areas for setting volume
        for i in 0..=volume_bar.width {
            let volume_area = Rect::new(
                volume_bar.x.saturating_add(i),
                volume_bar.y,
                1,
                volume_bar.height,
            );

            let volume_step = 1.5 / volume_bar.width as f32;
            let volume = volume_step * i as f32;
            // Make the volume sticky around 100%. Otherwise it's often not
            // possible to select by mouse.
            let sticky_volume = if (1.0 - volume).abs() <= volume_step {
                1.0
            } else {
                volume
            };

            mouse_areas.push((
                volume_area,
                vec![
                    MouseEventKind::Down(MouseButton::Left),
                    MouseEventKind::Drag(MouseButton::Left),
                ],
                vec![
                    Action::SelectObject(self.node.id),
                    Action::SetAbsoluteVolume(sticky_volume),
                ],
            ));
        }

        // Render peaks
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
