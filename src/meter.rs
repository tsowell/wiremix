use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect, Widget},
    style::{Color, Style},
    text::{Line, Span},
};

fn render_peak(peak: f32, area: Rect) -> (String, String, String) {
    // Convert to dB between -20 and +6
    let db = 20.0 * (peak + 1e-10).log10();
    let reference_offset: f32 = 0.0;
    let vu_value = (db + reference_offset).clamp(-20.0, 6.0);

    // Percentage at which to put 0 dB
    let zero = 0.75;

    // Scale to between 0.0 and 1.0, compressing positive values
    let meter = if vu_value < 0.0 {
        0.0 + ((vu_value + 20.0) / 20.0) * zero
    } else {
        zero + (vu_value / 6.0) * (1.0 - zero)
    };

    let total_chars = area.width as usize;

    // The character position for 0 dB
    let zero_char = (zero * total_chars as f32).round() as usize;
    let lit_chars =
        ((meter * total_chars as f32).round() as usize).min(total_chars);

    // Assign colors
    let green_chars = lit_chars.min(zero_char);
    let red_chars = lit_chars.saturating_sub(zero_char);
    let unlit_chars = total_chars - green_chars - red_chars;

    let ch = "▮";

    (
        ch.repeat(green_chars),
        ch.repeat(red_chars),
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
    let mono_peak = peak.unwrap_or_default();

    let area = meter_area;
    let (lit_peak, hilit_peak, unlit_peak) = render_peak(mono_peak, area);
    Line::from(vec![
        Span::styled(lit_peak, Style::default().fg(Color::LightGreen)),
        Span::styled(hilit_peak, Style::default().fg(Color::Red)),
        Span::styled(unlit_peak, Style::default().fg(Color::DarkGray)),
    ])
    .render(area, buf);
}
