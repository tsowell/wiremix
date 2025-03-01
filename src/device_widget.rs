use ratatui::{
    layout::Flex,
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

use crossterm::event::{MouseButton, MouseEventKind};

use crate::app::{Action, MouseArea};
use crate::object_list::ObjectList;
use crate::view;

pub struct DeviceWidget<'a> {
    device: &'a view::Device,
    selected: bool,
}

impl<'a> DeviceWidget<'a> {
    pub fn new(device: &'a view::Device, selected: bool) -> Self {
        Self { device, selected }
    }

    /// Height of a full device display.
    pub fn height() -> u16 {
        3
    }

    /// Spacing between objects
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

        // Position the popup so that the first item is over the displayed item
        let x = list_area.left().saturating_add(4);
        let y = object_area.top().saturating_add(1);
        // Add 2 for vertical borders and 2 for highlight symbol
        let width = max_target_length.saturating_add(4) as u16;
        let height = std::cmp::min(max_visible_items, object_list.targets.len())
            .saturating_add(2) as u16; // Add 2 for horizontal borders

        Rect::new(x, y, width, height)
    }
}

impl StatefulWidget for DeviceWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        mouse_areas.push((
            area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::SelectObject(self.device.id)],
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
                Constraint::Length(1), // title_area
                Constraint::Length(1), // target_area
            ])
            .spacing(1)
            .flex(Flex::Legacy)
            .split(node_area);
        let title_area = layout[0];
        let target_area = layout[1];

        Line::from(format!("   {}", self.device.title)).render(title_area, buf);

        Line::from(format!("    ▼ {}", self.device.target_title))
            .render(target_area, buf);
        mouse_areas.push((
            target_area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::SelectObject(self.device.id), Action::OpenPopup],
        ));
    }
}
