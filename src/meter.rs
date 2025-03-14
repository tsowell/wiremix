//! Peak level meter rendering.

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    text::{Line, Span},
};

use crate::config::Config;

fn render_peak(peak: f32, area: Rect) -> (usize, usize, usize) {
    fn normalize(value: f32) -> f32 {
        let amplitude = 10.0_f32.powf(value / 60.0);
        let min = 10.0_f32.powf(-60.0 / 60.0);
        let max = 10.0_f32.powf(6.0 / 60.0);

        (amplitude - min) / (max - min)
    }

    // Convert to dB between -20 and +3
    let db = 20.0 * (peak + 1e-10).log10();
    let vu_value = db.clamp(-60.0, 6.0);

    let meter = normalize(vu_value);

    let total_chars = area.width as usize;
    let lit = ((meter * total_chars as f32).round() as usize).min(total_chars);

    // Values above 0.0 will be colored differently
    let zero_char = (normalize(0.0) * total_chars as f32).round() as usize;

    // Assign colors
    let active_size = lit.min(zero_char);
    let overload_size = lit.saturating_sub(zero_char);
    let inactive_size = total_chars
        .saturating_sub(active_size)
        .saturating_sub(overload_size);

    (active_size, overload_size, inactive_size)
}

pub fn render_stereo(
    meter_area: Rect,
    buf: &mut Buffer,
    peaks: Option<(f32, f32)>,
    config: &Config,
) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(2),   // meter_left
            Constraint::Length(2), // meter_live
            Constraint::Fill(2),   // meter_right
        ])
        .spacing(1)
        .split(meter_area);
    let meter_left = layout[0];
    let meter_live = layout[1];
    let meter_right = layout[2];

    let (left_peak, right_peak) = peaks.unwrap_or_default();

    let area = meter_left;
    let (active_peak, overload_peak, inactive_peak) =
        render_peak(left_peak, area);
    Line::from(vec![
        Span::styled(
            config.char_set.meter_left_inactive.repeat(inactive_peak),
            config.theme.meter_inactive,
        ),
        Span::styled(
            config.char_set.meter_left_overload.repeat(overload_peak),
            config.theme.meter_overload,
        ),
        Span::styled(
            config.char_set.meter_left_active.repeat(active_peak),
            config.theme.meter_active,
        ),
    ])
    .alignment(Alignment::Right)
    .render(area, buf);

    let area = meter_right;
    let (active_peak, overload_peak, inactive_peak) =
        render_peak(right_peak, area);
    Line::from(vec![
        Span::styled(
            config.char_set.meter_right_active.repeat(active_peak),
            config.theme.meter_active,
        ),
        Span::styled(
            config.char_set.meter_right_overload.repeat(overload_peak),
            config.theme.meter_overload,
        ),
        Span::styled(
            config.char_set.meter_right_inactive.repeat(inactive_peak),
            config.theme.meter_inactive,
        ),
    ])
    .render(area, buf);

    let live_line = if peaks.is_some() {
        Line::from(Span::styled(
            format!(
                "{}{}",
                &config.char_set.meter_center_left_active,
                &config.char_set.meter_center_right_active,
            ),
            config.theme.meter_center_active,
        ))
    } else {
        Line::from(Span::styled(
            format!(
                "{}{}",
                &config.char_set.meter_center_left_inactive,
                &config.char_set.meter_center_right_inactive
            ),
            config.theme.meter_center_inactive,
        ))
    };
    live_line.render(meter_live, buf);
}

pub fn render_mono(
    meter_area: Rect,
    buf: &mut Buffer,
    peak: Option<f32>,
    config: &Config,
) {
    let mono_peak = peak.unwrap_or_default();

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(1), // meter_live
            Constraint::Fill(2),   // meter_mono
        ])
        .spacing(1)
        .split(meter_area);
    let meter_live = layout[0];
    let meter_mono = layout[1];

    let area = meter_mono;
    let (active_peak, overload_peak, inactive_peak) =
        render_peak(mono_peak, area);
    Line::from(vec![
        Span::styled(
            config.char_set.meter_right_active.repeat(active_peak),
            config.theme.meter_active,
        ),
        Span::styled(
            config.char_set.meter_right_overload.repeat(overload_peak),
            config.theme.meter_overload,
        ),
        Span::styled(
            config.char_set.meter_right_inactive.repeat(inactive_peak),
            config.theme.meter_inactive,
        ),
    ])
    .render(area, buf);

    let live_line = if peak.is_some() {
        Line::from(Span::styled(
            &config.char_set.meter_center_right_active,
            config.theme.meter_center_active,
        ))
    } else {
        Line::from(Span::styled(
            &config.char_set.meter_center_right_inactive,
            config.theme.meter_center_inactive,
        ))
    };
    live_line.render(meter_live, buf);
}
