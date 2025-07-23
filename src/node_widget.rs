//! A Ratatui widget representing a single PipeWire node in an object list.

use ratatui::{
    layout::Flex,
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

use crossterm::event::{MouseButton, MouseEventKind};
use smallvec::smallvec;

use crate::app::{Action, MouseArea};
use crate::config::{Config, Peaks};
use crate::device_kind::DeviceKind;
use crate::meter;
use crate::object_list::ObjectList;
use crate::truncate;
use crate::view;

fn is_default(node: &view::Node, device_kind: Option<DeviceKind>) -> bool {
    match device_kind {
        Some(DeviceKind::Sink) => node.is_default_sink,
        Some(DeviceKind::Source) => node.is_default_source,
        None => false,
    }
}

fn node_title(node: &view::Node, device_kind: Option<DeviceKind>) -> &str {
    match (device_kind, &node.title_source_sink) {
        (
            Some(DeviceKind::Source | DeviceKind::Sink),
            Some(title_source_sink),
        ) => title_source_sink,
        _ => &node.title,
    }
}

pub struct NodeWidget<'a> {
    config: &'a Config,
    device_kind: Option<DeviceKind>,
    node: &'a view::Node,
    selected: bool,
}

impl<'a> NodeWidget<'a> {
    pub fn new(
        config: &'a Config,
        device_kind: Option<DeviceKind>,
        node: &'a view::Node,
        selected: bool,
    ) -> Self {
        Self {
            config,
            device_kind,
            node,
            selected,
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

    /// Area for the target dropdown
    pub fn dropdown_area(
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

        mouse_areas.extend([
            (
                area,
                smallvec![MouseEventKind::Down(MouseButton::Left)],
                smallvec![Action::SelectObject(self.node.object_id)],
            ),
            (
                area,
                smallvec![MouseEventKind::Down(MouseButton::Right)],
                smallvec![
                    Action::SelectObject(self.node.object_id),
                    Action::SetDefault
                ],
            ),
            (
                area,
                smallvec![MouseEventKind::ScrollLeft],
                smallvec![
                    Action::SelectObject(self.node.object_id),
                    Action::SetRelativeVolume(-0.01),
                ],
            ),
            (
                area,
                smallvec![MouseEventKind::ScrollRight],
                smallvec![
                    Action::SelectObject(self.node.object_id),
                    Action::SetRelativeVolume(0.01),
                ],
            ),
        ]);

        // Split area into a selection indicator on the left and the main node
        // area on the right
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1), // selector_area
                Constraint::Min(0),    // node_area
            ])
            .split(area);
        let selector_area = layout[0];
        let node_area = layout[1];

        SelectorWidget::new(self.config, self.selected)
            .render(selector_area, buf);

        // Split the main node area into a header line and a line for the
        // volume bar and peak meter.
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

        HeaderWidget::new(self.config, self.device_kind, self.node).render(
            header_area,
            buf,
            mouse_areas,
        );

        // Render volume bar and (if enabled) peak meter
        let volume = VolumeWidget::new(self.config, self.node);
        if self.config.peaks == Peaks::Off {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
                    Constraint::Length(2), // _padding
                    Constraint::Fill(9),   // volume_area
                    Constraint::Fill(1),   // _padding
                ])
                .split(bar_area);
            // index 0 is _padding
            let volume_area = layout[1];

            volume.render(volume_area, buf, mouse_areas);
        } else {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![
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

            volume.render(volume_area, buf, mouse_areas);
            MeterWidget::new(self.config, self.node).render(meter_area, buf);
        }
    }
}

struct SelectorWidget<'a> {
    config: &'a Config,
    selected: bool,
}

impl<'a> SelectorWidget<'a> {
    fn new(config: &'a Config, selected: bool) -> Self {
        Self { config, selected }
    }
}

impl Widget for SelectorWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.selected {
            // Render and indication that this is the selected node.
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(area);

            let style = self.config.theme.selector;

            // Render the selected node indicator
            Span::styled(&self.config.char_set.selector_top, style)
                .render(rows[0], buf);
            Span::styled(&self.config.char_set.selector_middle, style)
                .render(rows[1], buf);
            Span::styled(&self.config.char_set.selector_bottom, style)
                .render(rows[2], buf);
        }
    }
}

struct HeaderWidget<'a> {
    config: &'a Config,
    device_kind: Option<DeviceKind>,
    node: &'a view::Node,
}

impl<'a> HeaderWidget<'a> {
    fn new(
        config: &'a Config,
        device_kind: Option<DeviceKind>,
        node: &'a view::Node,
    ) -> Self {
        Self {
            config,
            device_kind,
            node,
        }
    }

    fn target_line(&self) -> Line {
        match self.node.target {
            Some(view::Target::Default) => {
                // Add the default target indicator
                Line::from(vec![
                    Span::styled(
                        &self.config.char_set.default_stream,
                        self.config.theme.default_stream,
                    ),
                    Span::from(" "),
                    Span::styled(
                        &self.node.target_title,
                        self.config.theme.node_target,
                    ),
                ])
            }
            _ => Line::from(Span::styled(
                &self.node.target_title,
                self.config.theme.node_target,
            )),
        }
    }

    fn title_line(&self, width: usize) -> Line {
        let node_title = node_title(self.node, self.device_kind);
        let default_span = if is_default(self.node, self.device_kind) {
            Span::styled(
                &self.config.char_set.default_device,
                self.config.theme.default_device,
            )
        } else {
            Span::from(" ")
        };
        let node_title = truncate::with_ellipses(node_title, width);
        Line::from(vec![
            default_span,
            Span::from(" "),
            Span::styled(node_title, self.config.theme.node_title),
        ])
    }
}

impl StatefulWidget for HeaderWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        let target_line = self.target_line();

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(0),                             // header_left
                Constraint::Length(target_line.width() as u16), // header_right
            ])
            .horizontal_margin(1)
            .spacing(1)
            .split(area);
        let header_left = layout[0];
        let header_right = layout[1];

        target_line
            .alignment(Alignment::Right)
            .render(header_right, buf);
        mouse_areas.push((
            header_right,
            smallvec![MouseEventKind::Down(MouseButton::Left)],
            smallvec![
                Action::SelectObject(self.node.object_id),
                Action::ActivateDropdown
            ],
        ));

        self.title_line((header_left.width.saturating_sub(2)) as usize)
            .render(header_left, buf);
    }
}

struct VolumeWidget<'a> {
    config: &'a Config,
    node: &'a view::Node,
}

impl<'a> VolumeWidget<'a> {
    fn new(config: &'a Config, node: &'a view::Node) -> Self {
        Self { config, node }
    }
}

impl StatefulWidget for VolumeWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        let max_volume = self.config.max_volume_percent / 100.0;

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(5), // volume_label
                Constraint::Min(0),    // volume_bar
            ])
            .spacing(1)
            .split(area);
        let volume_label = layout[0];
        let volume_bar = layout[1];

        let volumes = &self.node.volumes;
        if !volumes.is_empty() {
            let mean = volumes.iter().sum::<f32>() / volumes.len() as f32;
            let volume = mean.cbrt();
            let percent = (volume * 100.0).round() as u32;

            Line::from(Span::styled(
                format!("{percent}%"),
                self.config.theme.volume,
            ))
            .alignment(Alignment::Right)
            .render(volume_label, buf);

            let count = ((volume.clamp(0.0, max_volume) / max_volume)
                * volume_bar.width as f32)
                .round() as usize;

            let filled = self.config.char_set.volume_filled.repeat(count);
            let blank = self
                .config
                .char_set
                .volume_empty
                .repeat((volume_bar.width as usize).saturating_sub(count));
            Line::from(vec![
                Span::styled(filled, self.config.theme.volume_filled),
                Span::styled(blank, self.config.theme.volume_empty),
            ])
            .render(volume_bar, buf);
        }
        if self.node.mute {
            Line::from("muted").render(volume_label, buf);
        }

        mouse_areas.push((
            volume_label,
            smallvec![MouseEventKind::Down(MouseButton::Left)],
            smallvec![
                Action::SelectObject(self.node.object_id),
                Action::ToggleMute
            ],
        ));

        // Add mouse areas for setting volume
        for i in 0..=volume_bar.width {
            let volume_area = Rect::new(
                volume_bar.x.saturating_add(i),
                volume_bar.y,
                1,
                volume_bar.height,
            );

            let volume_step = max_volume / volume_bar.width as f32;
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
                smallvec![
                    MouseEventKind::Down(MouseButton::Left),
                    MouseEventKind::Drag(MouseButton::Left),
                ],
                smallvec![
                    Action::SelectObject(self.node.object_id),
                    Action::SetAbsoluteVolume(sticky_volume),
                ],
            ));
        }
    }
}

struct MeterWidget<'a> {
    config: &'a Config,
    node: &'a view::Node,
}

impl<'a> MeterWidget<'a> {
    fn new(config: &'a Config, node: &'a view::Node) -> Self {
        Self { config, node }
    }
}

impl Widget for MeterWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.node.peaks.as_deref() {
            Some([left, right]) if self.config.peaks != Peaks::Mono => {
                meter::render_stereo(
                    area,
                    buf,
                    Some((*left, *right)),
                    self.config,
                )
            }
            Some(peaks @ [..]) => meter::render_mono(
                area,
                buf,
                (!peaks.is_empty())
                    .then_some(peaks.iter().sum::<f32>() / peaks.len() as f32),
                self.config,
            ),
            _ => match self
                .node
                .positions
                .as_ref()
                .map(|positions| positions.len())
            {
                Some(2) if self.config.peaks != Peaks::Mono => {
                    meter::render_stereo(area, buf, None, self.config)
                }
                _ => meter::render_mono(area, buf, None, self.config),
            },
        }
    }
}
