use ratatui::{
    prelude::{Alignment, Buffer, Rect, Widget},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, StatefulWidget},
};

use crossterm::event::{MouseButton, MouseEventKind};

use crate::app::{Action, MouseArea};
use crate::object_list::ObjectList;

pub struct PopupWidget<'a> {
    object_list: &'a mut ObjectList,
    popup_area: &'a Rect,
}

impl<'a> PopupWidget<'a> {
    pub fn new(object_list: &'a mut ObjectList, popup_area: &'a Rect) -> Self {
        Self {
            object_list,
            popup_area,
        }
    }
}

impl StatefulWidget for PopupWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        let targets: Vec<_> = self
            .object_list
            .targets
            .iter()
            .map(|(_, title)| title.clone())
            .collect();

        let popup_area = self.popup_area.clamp(area);

        // Click anywhere else in the object list to close the popup.
        mouse_areas.push((
            area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::ClosePopup],
        ));

        // But clicking on the border does nothing.
        mouse_areas.push((
            popup_area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![],
        ));

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

        let first_index = self.object_list.list_state.offset();

        if first_index > 0 {
            let top_area =
                Rect::new(popup_area.x, popup_area.y, popup_area.width, 1);

            Line::from(Span::styled(
                "•••",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Center)
            .render(top_area, buf);

            mouse_areas.push((
                top_area,
                vec![MouseEventKind::Down(MouseButton::Left)],
                vec![Action::ScrollUp],
            ));
        }

        let last_index = first_index + popup_area.height as usize - 2;
        if last_index < self.object_list.targets.len() {
            let bottom_area = Rect::new(
                popup_area.x,
                popup_area.y + popup_area.height - 1,
                popup_area.width,
                1,
            );

            Line::from(Span::styled(
                "•••",
                Style::default().fg(Color::DarkGray),
            ))
            .alignment(Alignment::Center)
            .render(bottom_area, buf);

            mouse_areas.push((
                bottom_area,
                vec![MouseEventKind::Down(MouseButton::Left)],
                vec![Action::ScrollDown],
            ));
        }

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
                .skip(first_index)
                .nth(i as usize)
                .map(|(target, _)| target);
            if let Some(target) = target {
                mouse_areas.push((
                    target_area,
                    vec![MouseEventKind::Down(MouseButton::Left)],
                    vec![Action::SetTarget(*target)],
                ));
            }
        }
    }
}
