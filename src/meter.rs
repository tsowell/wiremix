use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    text::{Line, Span},
};

fn render_peak(peak: f32, area: Rect) -> (String, String, String) {
    fn normalize(value: f32) -> f32 {
        (value + 20.0) / 23.0
    }

    // Convert to dB between -20 and +3
    let db = 20.0 * (peak + 1e-10).log10();
    let vu_value = db.clamp(-20.0, 3.0);

    let meter = normalize(vu_value);

    let total_chars = area.width as usize;
    let lit_chars =
        ((meter * total_chars as f32).round() as usize).min(total_chars);

    // Values above 0.0 will be colored differently
    let zero_char = (normalize(0.0) * total_chars as f32).round() as usize;

    // Assign colors
    let normal_chars = lit_chars.min(zero_char);
    let overload_chars = lit_chars.saturating_sub(zero_char);
    let unlit_chars = total_chars
        .saturating_sub(normal_chars)
        .saturating_sub(overload_chars);

    let ch = "▮";

    (
        ch.repeat(normal_chars),
        ch.repeat(overload_chars),
        ch.repeat(unlit_chars),
    )
}

pub fn render_stereo(
    meter_area: Rect,
    buf: &mut Buffer,
    peaks: Option<(f32, f32)>,
) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(2),   // meter_left
            Constraint::Length(2), // meter_center
            Constraint::Fill(2),   // meter_right
        ])
        .spacing(1)
        .split(meter_area);
    let meter_left = layout[0];
    let meter_center = layout[1];
    let meter_right = layout[2];

    let (left_peak, right_peak) = peaks.unwrap_or_default();

    let area = meter_left;
    let (normal_peak, overload_peak, unlit_peak) = render_peak(left_peak, area);
    Line::from(vec![
        Span::styled(unlit_peak, Style::default().fg(Color::DarkGray)),
        Span::styled(overload_peak, Style::default().fg(Color::Red)),
        Span::styled(normal_peak, Style::default().fg(Color::LightGreen)),
    ])
    .alignment(Alignment::Right)
    .render(area, buf);

    let area = meter_right;
    let (normal_peak, overload_peak, unlit_peak) =
        render_peak(right_peak, area);
    Line::from(vec![
        Span::styled(normal_peak, Style::default().fg(Color::LightGreen)),
        Span::styled(overload_peak, Style::default().fg(Color::Red)),
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
    let mono_peak = peak.unwrap_or_default();

    let area = meter_area;
    let (normal_peak, overload_peak, unlit_peak) = render_peak(mono_peak, area);
    Line::from(vec![
        Span::styled(normal_peak, Style::default().fg(Color::LightGreen)),
        Span::styled(overload_peak, Style::default().fg(Color::Red)),
        Span::styled(unlit_peak, Style::default().fg(Color::DarkGray)),
    ])
    .render(area, buf);
}
