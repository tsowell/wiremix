use ratatui::{
    layout::Flex,
    prelude::{Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{
        Block, BorderType, Borders, Clear, List, Padding, StatefulWidget,
        Widget,
    },
};

use crate::app::Action;
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
}

impl StatefulWidget for DeviceWidget<'_> {
    type State = Vec<(Rect, Vec<Action>)>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let click_areas = state;

        click_areas.push((area, vec![Action::SelectObject(self.device.id)]));

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
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut target_area)),
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

        Line::from(format!("   {}", self.device.title)).render(title_area, buf);

        Line::from(format!("    ▼ {}", self.device.target_title))
            .render(target_area, buf);
        click_areas.push((
            target_area,
            vec![Action::SelectObject(self.device.id), Action::OpenPopup],
        ));
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

impl StatefulWidget for DevicePopupWidget<'_> {
    type State = Vec<(Rect, Vec<Action>)>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let click_areas = state;

        let targets: Vec<_> = self
            .object_list
            .targets
            .iter()
            .map(|(_, title)| title.clone())
            .collect();
        let max_target_length =
            targets.iter().map(|s| s.len()).max().unwrap_or(0);

        let popup_area = Rect::new(
            self.list_area.left() + 6,
            area.top() + 1,
            max_target_length as u16 + 2,
            std::cmp::min(7, targets.len() as u16 + 2),
        )
        .clamp(*self.parent_area);

        // Click anywhere else in the object list to close the popup.
        click_areas.push((*self.parent_area, vec![Action::ClosePopup]));

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

        for i in 0..(popup_area.height - 2) {
            let target_area = Rect::new(
                popup_area.x,
                popup_area.y + 1 + i,
                popup_area.width,
                1,
            );

            let target = self
                .object_list
                .targets
                .iter()
                .skip(self.object_list.list_state.offset())
                .nth(i as usize)
                .map(|(target, _)| target);
            if let Some(target) = target {
                click_areas
                    .push((target_area, vec![Action::SetTarget(*target)]));
            }
        }
    }
}
