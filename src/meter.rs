use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    text::{Line, Span},
};

use crate::named_constraints::with_named_constraints;

fn render_peak(peak: f32, area: Rect) -> (String, String, String) {
    let peak = peak.cbrt();
    let total_width = area.width as usize;
    let lit_width = (peak * area.width as f32) as usize;

    let hilit_width =
        ((peak - 0.70).clamp(0.0, 1.0) * area.width as f32) as usize;

    let unlit_width = total_width.saturating_sub(lit_width);
    let lit_width = lit_width.saturating_sub(hilit_width);

    let ch = "▮";

    (
        ch.repeat(lit_width),
        ch.repeat(hilit_width),
        ch.repeat(unlit_width),
    )
}

pub fn render_stereo(
    meter_area: Rect,
    buf: &mut Buffer,
    peaks: Option<(f32, f32)>,
) {
    let mut meter_left = Default::default();
    let mut meter_center = Default::default();
    let mut meter_right = Default::default();
    with_named_constraints!(
        [
            (Constraint::Fill(2), Some(&mut meter_left)),
            (Constraint::Length(1), None),
            (Constraint::Length(2), Some(&mut meter_center)),
            (Constraint::Length(1), None),
            (Constraint::Fill(2), Some(&mut meter_right)),
        ],
        |constraints| {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(meter_area)
        }
    );

    let (left_peak, right_peak) = peaks.unwrap_or_default();

    let area = meter_left;
    let (lit_peak, hilit_peak, unlit_peak) = render_peak(left_peak, area);
    Line::from(vec![
        Span::styled(unlit_peak, Style::default().fg(Color::DarkGray)),
        Span::styled(hilit_peak, Style::default().fg(Color::Red)),
        Span::styled(lit_peak, Style::default().fg(Color::LightGreen)),
    ])
    .alignment(Alignment::Right)
    .render(area, buf);

    let area = meter_right;
    let (lit_peak, hilit_peak, unlit_peak) = render_peak(right_peak, area);
    Line::from(vec![
        Span::styled(lit_peak, Style::default().fg(Color::LightGreen)),
        Span::styled(hilit_peak, Style::default().fg(Color::Red)),
        Span::styled(unlit_peak, Style::default().fg(Color::DarkGray)),
    ])
    .render(area, buf);

    let center_color = if peaks.is_some() {
        Color::LightGreen
    } else {
        Color::DarkGray
    };
    Line::from(Span::styled(
        "■■".to_string(),
        Style::default().fg(center_color),
    ))
    .render(meter_center, buf);
}

pub fn render_mono(meter_area: Rect, buf: &mut Buffer, peak: Option<f32>) {
    let mut meter_head = Default::default();
    let mut meter_rest = Default::default();
    with_named_constraints!(
        [
            (Constraint::Length(2), Some(&mut meter_head)),
            (Constraint::Min(0), Some(&mut meter_rest)),
        ],
        |constraints| {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(constraints)
                .split(meter_area)
        }
    );

    let mono_peak = peak.unwrap_or_default();

    let area = meter_rest;
    let (lit_peak, hilit_peak, unlit_peak) = render_peak(mono_peak, area);
    Line::from(vec![
        Span::styled(lit_peak, Style::default().fg(Color::LightGreen)),
        Span::styled(hilit_peak, Style::default().fg(Color::Red)),
        Span::styled(unlit_peak, Style::default().fg(Color::DarkGray)),
    ])
    .render(area, buf);

    let center_color = if peak.is_some() {
        Color::LightGreen
    } else {
        Color::DarkGray
    };
    Line::from(Span::styled(
        "■".to_string(),
        Style::default().fg(center_color),
    ))
    .render(meter_head, buf);
}
