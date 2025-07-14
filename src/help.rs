use crossterm::event::{MouseButton, MouseEventKind};
use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Rect, Widget},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Row, StatefulWidget, Table},
};
use smallvec::smallvec;

use crate::app::{Action, MouseArea};
use crate::config::Config;

pub struct HelpWidget<'a> {
    pub config: &'a Config,
}

pub struct HelpWidgetState<'a> {
    pub mouse_areas: &'a mut Vec<MouseArea>,
    pub help_position: &'a mut u16,
}

impl HelpWidget<'_> {
    const BORDER_WIDTH: usize = 1;
    const BORDER_PADDING: u16 = 2;
    const COLUMN_PADDING: u16 = 2;

    pub fn base_width() -> usize {
        // * 2 because there are 2 horizontal borders
        Self::BORDER_WIDTH * 2
            + (Self::BORDER_PADDING as usize * 2)
            + (Self::COLUMN_PADDING as usize)
    }
}

impl<'a> StatefulWidget for HelpWidget<'a> {
    type State = HelpWidgetState<'a>;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // App initialized mouse_areas so clicking anywhere closes this widget.
        // Make it safe to click within the widget.
        state.mouse_areas.push((
            area,
            smallvec![MouseEventKind::Down(MouseButton::Left)],
            smallvec![Action::Nothing],
        ));

        let borders = Block::default()
            .borders(Borders::ALL)
            .border_style(self.config.theme.help_border)
            .border_type(self.config.char_set.help_border)
            .padding(Padding::horizontal(Self::BORDER_PADDING));

        let list_area = borders.inner(area);
        borders.render(area, buf);

        state.mouse_areas.push((
            list_area,
            smallvec![MouseEventKind::ScrollUp],
            smallvec![Action::MoveUp],
        ));
        state.mouse_areas.push((
            list_area,
            smallvec![MouseEventKind::ScrollDown],
            smallvec![Action::MoveDown],
        ));

        // Fix help_position if we are scrolled beyond the bottom of the list
        let rows_total = self.config.help.rows.len();
        {
            let rows_visible =
                rows_total.saturating_sub((*state.help_position).into());
            if rows_visible < list_area.height.into() {
                *state.help_position = rows_total
                    .saturating_sub(list_area.height.into())
                    .try_into()
                    .unwrap_or(u16::MAX);
            }
        }

        // Add a clickable indicator to the top border if there are more items
        if *state.help_position > 0 {
            let top_area = Rect::new(area.x, area.y, area.width, 1);

            Line::from(Span::styled(
                &self.config.char_set.help_more,
                self.config.theme.help_more,
            ))
            .alignment(Alignment::Center)
            .render(top_area, buf);

            state.mouse_areas.push((
                top_area,
                smallvec![MouseEventKind::Down(MouseButton::Left)],
                smallvec![Action::MoveUp],
            ));
        }

        // Add a clickable indiciator to the bottom border if there are more
        // items
        if usize::from(*state.help_position + list_area.height) < rows_total {
            let y = area.y.saturating_add(area.height.saturating_sub(1));
            let bottom_area = Rect::new(area.x, y, area.width, 1);

            Line::from(Span::styled(
                &self.config.char_set.help_more,
                self.config.theme.help_more,
            ))
            .alignment(Alignment::Center)
            .render(bottom_area, buf);

            state.mouse_areas.push((
                bottom_area,
                smallvec![MouseEventKind::Down(MouseButton::Left)],
                smallvec![Action::MoveDown],
            ));
        }

        let rows: Vec<Row> = self
            .config
            .help
            .rows
            .iter()
            .skip((*state.help_position).into())
            .map(|row| Row::new(row.clone()))
            .collect();

        let widths: Vec<Constraint> = self
            .config
            .help
            .widths
            .iter()
            .map(|width| {
                Constraint::Max((*width).try_into().unwrap_or(u16::MAX))
            })
            .collect();
        let table = Table::new(rows, widths)
            .style(self.config.theme.help_item)
            .column_spacing(Self::COLUMN_PADDING);
        Widget::render(table, list_area, buf);
    }
}
