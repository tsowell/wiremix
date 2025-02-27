use ratatui::{
    layout::Flex,
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{StatefulWidget, Widget},
};

use crossterm::event::{MouseButton, MouseEventKind};

use crate::app::{Action, MouseArea};
use crate::named_constraints::with_named_constraints;
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
        let max_target_length = object_list
            .targets
            .iter()
            .map(|(_, title)| title.len())
            .max()
            .unwrap_or(0);

        Rect::new(
            list_area.left() + 6,
            object_area.top() + 1,
            max_target_length as u16 + 2,
            std::cmp::min(7, object_list.targets.len() as u16 + 2),
        )
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

        let mut selected_area = Default::default();
        let mut node_area = Default::default();
        let _layout = with_named_constraints!(
            [
                (Constraint::Length(1), Some(&mut selected_area)),
                (Constraint::Min(0), Some(&mut node_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(area)
            }
        );

        if self.selected {
            let mut top = Default::default();
            let mut center = Default::default();
            let mut bottom = Default::default();
            let _layout = with_named_constraints!(
                [
                    (Constraint::Length(1), Some(&mut top)),
                    (Constraint::Length(1), Some(&mut center)),
                    (Constraint::Length(1), Some(&mut bottom)),
                ],
                |constraints| {
                    Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(constraints)
                        .split(selected_area)
                }
            );

            let style = Style::default().fg(Color::Cyan);

            Line::from(Span::styled("░", style)).render(top, buf);
            Line::from(Span::styled("▒", style)).render(center, buf);
            Line::from(Span::styled("░", style)).render(bottom, buf);
        }

        let mut title_area = Default::default();
        let mut target_area = Default::default();
        let _layout = with_named_constraints!(
            [
                (Constraint::Length(1), Some(&mut title_area)),
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut target_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .flex(Flex::Legacy)
                    .split(node_area)
            }
        );

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
