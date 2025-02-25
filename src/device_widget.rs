use ratatui::{
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{
        Block, BorderType, Borders, Clear, List, Padding, StatefulWidget,
        Widget,
    },
};

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
        4
    }

    /// Height of the important parts (excluding blank margin lines at bottom).
    pub fn important_height() -> u16 {
        4
    }
}

impl Widget for DeviceWidget<'_> {
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
        let mut title_area = Default::default();
        let mut target_area = Default::default();
        let _layout = with_named_constraints!(
            [
                (Constraint::Length(1), Some(&mut title_area)),
                (Constraint::Length(3), Some(&mut target_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(border_block.inner(area))
            }
        );
        border_block.render(area, buf);

        Line::from(format!(" {}", self.device.title)).render(title_area, buf);

        let target_block = Block::default().borders(Borders::ALL);
        (&target_block).render(target_area, buf);

        Line::from(format!(" {}", self.device.target_title))
            .render(target_block.inner(target_area), buf);
    }
}

pub struct DevicePopupWidget<'a> {
    object_list: &'a mut ObjectList,
    list_area: &'a Rect,
    parent_area: &'a Rect,
}

impl<'a> DevicePopupWidget<'a> {
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

impl Widget for DevicePopupWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let targets: Vec<_> = self
            .object_list
            .targets
            .iter()
            .map(|(_, title)| format!(" {}", title))
            .collect();

        let popup_area = Rect::new(
            self.list_area.left() + 1,
            area.top() + 1,
            self.list_area.width - 1,
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
