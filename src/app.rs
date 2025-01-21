use std::cell::RefCell;
use std::sync::mpsc;

use anyhow::{anyhow, Result};

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
    DefaultTerminal, Frame,
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

use crate::message::{InputMessage, Message};
use crate::state;

#[cfg(feature = "trace")]
use crate::{trace, trace_dbg};

thread_local! {
    pub static STATE: RefCell<state::State> = RefCell::new(Default::default());
}

pub struct App {
    exit: bool,
    rx: mpsc::Receiver<Message>,
    log: Vec<String>,
    error_message: Option<String>,
}

impl App {
    pub fn new(rx: mpsc::Receiver<Message>) -> Self {
        App {
            exit: Default::default(),
            rx,
            log: Default::default(),
            error_message: Default::default(),
        }
    }

    pub fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        #[cfg(feature = "trace")]
        trace::initialize_logging()?;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_messages()?;
        }

        self.error_message.map_or(Ok(()), |s| Err(anyhow!(s)))
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn exit(&mut self, error_message: Option<String>) {
        self.exit = true;
        self.error_message = error_message;
    }

    fn handle_messages(&mut self) -> Result<()> {
        // Block on getting the next message.
        self.handle_message(self.rx.recv()?)?;
        // Then handle the rest that are available.
        while let Ok(message) = self.rx.try_recv() {
            self.handle_message(message)?;
        }

        Ok(())
    }

    fn handle_message(&mut self, message: Message) -> Result<()> {
        if let Message::Input(InputMessage::Event(event)) = message {
            self.handle_event(event)
        } else if let Message::Error(error) = message {
            self.exit(Some(error));
            Ok(())
        } else if let Message::Monitor(message) = message {
            self.log.push(format!("{:?}", message));
            STATE.with_borrow_mut(|s| s.update(message));
            Ok(())
        } else {
            Ok(())
        }
    }

    fn handle_event(&mut self, event: Event) -> Result<()> {
        match event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => (),
        };

        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(None),
            _ => (),
        }
    }
}

macro_rules! with_named_constraints {
    ($constraints:expr, $closure:expr) => {{
        let mut vec = Vec::new();
        let mut names = Vec::new();
        let mut index = 0;
        for constraint in $constraints {
            match constraint {
                (constraint, Some::<&mut Rect>(var)) => {
                    names.push((var, index));
                    vec.push(constraint)
                }
                (constraint, None) => vec.push(constraint),
            }
            index += 1;
        }
        let layout = $closure(vec);
        for (var, index) in names {
            *var = layout[index];
        }
        layout
    }};
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        STATE.with_borrow(|state| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(state.nodes.iter().map(|_| Constraint::Length(5)))
                .split(area);
            for (node, area) in state.nodes.values().zip(layout.iter()) {
                node.render(*area, buf);
            }
        })
    }
}

fn truncate(text: &str, len: usize) -> String {
    if text.len() <= len {
        return text.to_string();
    }

    let left = len.saturating_sub(3);

    let truncated = text
        .char_indices()
        .take_while(|(i, _)| *i < left)
        .map(|(_, c)| c)
        .collect::<String>();

    truncated + &".".repeat(std::cmp::min(len, 3))
}

fn node_header_left(node: &state::Node) -> String {
    let default_string = STATE
        .with_borrow(|state| -> Option<String> {
            let metadata = state.get_metadata_by_name("default")?;
            let json = metadata.properties.get("default.audio.sink")?;
            let obj = serde_json::from_str::<serde_json::Value>(json).ok()?;
            let default_name = obj["name"].as_str()?;
            let node_name = node.name.as_ref()?;
            (default_name == node_name).then_some("⯁ ".to_string())
        })
        .unwrap_or_default();
    let title = match (&node.description, &node.name, &node.media_name) {
        (Some(description), _, _) => description.clone(),
        (None, Some(name), Some(media_name)) => format!("{name}: {media_name}"),
        _ => String::new(),
    };
    format!("{}{}", default_string, title)
}

fn node_header_right(node: &state::Node) -> String {
    let Some(ref media_class) = node.media_class else {
        return Default::default();
    };
    match media_class.as_str() {
        "Audio/Sink" | "Audio/Source" => STATE
            .with_borrow(|state| -> Option<String> {
                let device_id = node.device_id?;
                let device = state.devices.get(&device_id)?;
                let route_index = device.route_index?;
                let route = device.routes.get(&route_index)?;
                Some(route.description.clone())
            })
            .unwrap_or_default(),
        "Stream/Output/Audio" => STATE
            .with_borrow(|state| -> Option<String> {
                let outputs = state.links.get(&node.id)?;
                for output in outputs {
                    let Some(output_node) = state.nodes.get(output) else {
                        continue;
                    };
                    let Some(ref media_class) = output_node.media_class else {
                        continue;
                    };
                    if media_class != "Audio/Sink" {
                        continue;
                    };
                    let description = output_node.description.as_ref()?;
                    return Some(description.to_owned());
                }

                None
            })
            .unwrap_or_default(),
        _ => Default::default(),
    }
}

impl Widget for &state::Node {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_block = Block::default().borders(Borders::NONE);
        let mut header_area = Default::default();
        let mut bar_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut header_area)),
                (Constraint::Length(1), None),
                (Constraint::Length(1), Some(&mut bar_area)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(border_block.inner(area))
            }
        );
        border_block.render(area, buf);

        let left = node_header_left(self);
        let right = node_header_right(self);

        let mut header_left = Default::default();
        let mut header_right = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), None),
                (Constraint::Min(0), Some(&mut header_left)),
                (Constraint::Length(1), None), // Padding
                (
                    Constraint::Length(right.len() as u16),
                    Some(&mut header_right)
                ),
                (Constraint::Length(1), None),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(header_area)
            }
        );

        Line::from(right)
            .alignment(Alignment::Right)
            .render(header_right, buf);
        let left = truncate(&left, header_left.width as usize);
        Line::from(left).render(header_left, buf);

        let mut volume_area = Default::default();
        let mut meter_area = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(1), None),
                (Constraint::Fill(4), Some(&mut volume_area)),
                (Constraint::Fill(1), None),
                (Constraint::Fill(4), Some(&mut meter_area)),
                (Constraint::Fill(1), None),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(bar_area)
            }
        );

        let mut volume_label = Default::default();
        let mut volume_bar = Default::default();
        with_named_constraints!(
            [
                (Constraint::Length(4), Some(&mut volume_label)),
                (Constraint::Length(1), None), // Padding
                (Constraint::Min(0), Some(&mut volume_bar)),
            ],
            |constraints| {
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(volume_area)
            }
        );

        if let Some(volumes) = &self.volumes {
            if !volumes.is_empty() {
                let mean = volumes.iter().sum::<f32>() / volumes.len() as f32;
                let volume = mean.cbrt();
                let percent = (volume * 100.0) as u32;

                Line::from(format!("{}%", percent))
                    .alignment(Alignment::Right)
                    .render(volume_label, buf);

                let count = ((volume / 1.5) * volume_bar.width as f32) as usize;

                let filled = "━".repeat(count);
                let blank = "╌".repeat(volume_bar.width as usize - count);
                Line::from(vec![
                    Span::styled(filled, Style::default().fg(Color::Blue)),
                    Span::styled(blank, Style::default().fg(Color::Blue)),
                ])
                .render(volume_bar, buf);
            }
        }

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

        if let Some(peaks) = &self.peaks {
            if peaks.len() == 2 {
                fn render_peak(
                    peak: f32,
                    area: Rect,
                ) -> (String, String, String) {
                    let peak = peak.cbrt();
                    let total_width = area.width as usize;
                    let lit_width = (peak * area.width as f32) as usize;

                    let hilit_width = ((peak - 0.70).clamp(0.0, 1.0)
                        * area.width as f32)
                        as usize;

                    let unlit_width = total_width - lit_width;
                    let lit_width = lit_width - hilit_width;

                    let ch = "▮";

                    (
                        ch.repeat(lit_width),
                        ch.repeat(hilit_width),
                        ch.repeat(unlit_width),
                    )
                }

                let area = meter_left;
                let (lit_peak, hilit_peak, unlit_peak) =
                    render_peak(peaks[0], area);
                Line::from(vec![
                    Span::styled(
                        unlit_peak,
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(hilit_peak, Style::default().fg(Color::Red)),
                    Span::styled(
                        lit_peak,
                        Style::default().fg(Color::LightGreen),
                    ),
                ])
                .alignment(Alignment::Right)
                .render(area, buf);

                let area = meter_right;
                let (lit_peak, hilit_peak, unlit_peak) =
                    render_peak(peaks[1], area);
                Line::from(vec![
                    Span::styled(
                        lit_peak,
                        Style::default().fg(Color::LightGreen),
                    ),
                    Span::styled(hilit_peak, Style::default().fg(Color::Red)),
                    Span::styled(
                        unlit_peak,
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
                .render(area, buf);

                Line::from(Span::styled(
                    "■■".to_string(),
                    Style::default().fg(Color::LightGreen),
                ))
                .render(meter_center, buf);
            }
        } else {
            let ch = "▮";
            let area = meter_left;
            Line::from(Span::styled(
                ch.repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            ))
            .render(area, buf);
            let area = meter_right;
            Line::from(Span::styled(
                ch.repeat(area.width as usize),
                Style::default().fg(Color::DarkGray),
            ))
            .render(area, buf);

            Line::from(Span::styled(
                "■■".to_string(),
                Style::default().fg(Color::DarkGray),
            ))
            .render(meter_center, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_test_equal() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_test_larger() {
        assert_eq!(truncate("hello", 6), "hello");
    }

    #[test]
    fn truncate_test_shorter() {
        assert_eq!(truncate("hello", 4), "h...");
    }

    #[test]
    fn truncate_test_too_short() {
        assert_eq!(truncate("hello", 3), "...");
    }

    #[test]
    fn truncate_test_much_too_short() {
        assert_eq!(truncate("hello", 2), "..");
    }

    #[test]
    fn truncate_test_empty() {
        assert_eq!(truncate("hello", 0), "");
    }
}
