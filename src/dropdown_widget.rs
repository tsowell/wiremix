//! A Ratatui widget for a dropdown menu of options pertaining to a node or device
//! widget.

use ratatui::{
    prelude::{Alignment, Buffer, Rect, Widget},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, StatefulWidget},
};

use crossterm::event::{MouseButton, MouseEventKind};

use crate::app::{Action, MouseArea};
use crate::config::Config;
use crate::object_list::ObjectList;

pub struct DropdownWidget<'a> {
    object_list: &'a mut ObjectList,
    dropdown_area: &'a Rect,
    config: &'a Config,
}

impl<'a> DropdownWidget<'a> {
    pub fn new(
        object_list: &'a mut ObjectList,
        dropdown_area: &'a Rect,
        config: &'a Config,
    ) -> Self {
        Self {
            object_list,
            dropdown_area,
            config,
        }
    }
}

impl StatefulWidget for DropdownWidget<'_> {
    type State = Vec<MouseArea>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mouse_areas = state;

        let targets: Vec<_> = self
            .object_list
            .targets
            .iter()
            .map(|(_, title)| title.clone())
            .collect();

        let dropdown_area = self.dropdown_area.clamp(area);

        // Click anywhere else in the object list to close the dropdown.
        mouse_areas.push((
            area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![Action::CloseDropdown],
        ));

        // But clicking on the border does nothing.
        mouse_areas.push((
            dropdown_area,
            vec![MouseEventKind::Down(MouseButton::Left)],
            vec![],
        ));

        Clear.render(dropdown_area, buf);

        let highlight_symbol =
            format!("{} ", self.config.char_set.dropdown_selector);
        let list = List::new(targets)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(self.config.theme.dropdown_border)
                    .border_type(self.config.char_set.dropdown_border),
            )
            .style(self.config.theme.dropdown_item)
            .highlight_symbol(&highlight_symbol)
            .highlight_style(self.config.theme.dropdown_selected);

        StatefulWidget::render(
            &list,
            dropdown_area,
            buf,
            &mut self.object_list.list_state,
        );

        let first_index = self.object_list.list_state.offset();

        // Add a clickable indicator to the top border if there or more items
        // if scrolled up
        if first_index > 0 {
            let top_area = Rect::new(
                dropdown_area.x,
                dropdown_area.y,
                dropdown_area.width,
                1,
            );

            Line::from(Span::styled(
                &self.config.char_set.dropdown_more,
                self.config.theme.dropdown_more,
            ))
            .alignment(Alignment::Center)
            .render(top_area, buf);

            mouse_areas.push((
                top_area,
                vec![MouseEventKind::Down(MouseButton::Left)],
                vec![Action::MoveUp],
            ));
        }

        // Subtract 2 for vertical borders
        let dropdown_area_inner_height =
            (dropdown_area.height as usize).saturating_sub(2);
        let last_index = first_index.saturating_add(dropdown_area_inner_height);
        // Add a clickable indicator to the bottom border if there or more
        // items if scrolled down
        if last_index < self.object_list.targets.len() {
            let y = dropdown_area
                .y
                .saturating_add(dropdown_area.height.saturating_sub(1));
            let bottom_area =
                Rect::new(dropdown_area.x, y, dropdown_area.width, 1);

            Line::from(Span::styled(
                &self.config.char_set.dropdown_more,
                self.config.theme.dropdown_more,
            ))
            .alignment(Alignment::Center)
            .render(bottom_area, buf);

            mouse_areas.push((
                bottom_area,
                vec![MouseEventKind::Down(MouseButton::Left)],
                vec![Action::MoveDown],
            ));
        }

        for i in 0..(dropdown_area.height - 2) {
            let target_area = Rect::new(
                dropdown_area.x,
                dropdown_area.y.saturating_add(1).saturating_add(i),
                dropdown_area.width,
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
