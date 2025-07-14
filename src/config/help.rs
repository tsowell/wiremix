use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};

use crate::config::Action;

/// Keybinding help text.
///
/// Caches a tabular text representation of the configured keybindings.
#[derive(Debug)]
#[cfg_attr(test, derive(Default, PartialEq))]
pub struct Help {
    /// Human-readable help text in the form ["action", "keybinding"]
    pub rows: Vec<[String; 2]>,
    /// The max width of each column in [`Help::rows`]
    pub widths: [usize; 2],
}

impl From<&HashMap<KeyEvent, Action>> for Help {
    fn from(keybindings: &HashMap<KeyEvent, Action>) -> Self {
        let mut sorted: Vec<_> = keybindings
            .iter()
            .filter(|(_, action)| !matches!(action, Action::Nothing))
            .collect();
        sorted.sort_by(|(a_key, a_action), (b_key, b_action)| {
            a_action
                .partial_cmp(b_action)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a_key
                        .partial_cmp(b_key)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        let sorted = sorted;

        let rows = Self::generate_rows(&sorted);
        let widths = Self::calculate_widths(&rows);

        Self { rows, widths }
    }
}

impl Help {
    fn generate_rows(bindings: &[(&KeyEvent, &Action)]) -> Vec<[String; 2]> {
        let mut rows = Vec::new();
        let mut last_action = String::new();

        for (key, action) in bindings {
            let key_string = Self::format_key(key);
            let action_string = action.to_string();

            let action_display = if last_action == action_string {
                String::new() // Don't repeat the action name
            } else {
                action_string.clone()
            };

            rows.push([action_display, key_string]);
            last_action = action_string;
        }

        rows
    }

    fn format_key(key: &KeyEvent) -> String {
        let key_code_string = match key.code {
            KeyCode::BackTab => "Tab".to_string(),
            other => other.to_string(),
        };

        if key.modifiers.is_empty() {
            key_code_string
        } else {
            format!("{}+{}", key.modifiers, key_code_string)
        }
    }

    fn calculate_widths(rows: &[[String; 2]]) -> [usize; 2] {
        rows.iter().fold([0; 2], |mut acc, row| {
            for (i, string) in row.iter().enumerate() {
                acc[i] = acc[i].max(string.len());
            }
            acc
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use std::collections::HashMap;

    #[test]
    fn action_formatting() {
        assert_eq!(Action::Help.to_string(), "Show/hide help");
        assert_eq!(
            Action::SetRelativeVolume(0.01).to_string(),
            "Increment volume"
        );
        assert_eq!(
            Action::SetRelativeVolume(-0.01).to_string(),
            "Decrement volume"
        );
        assert_eq!(
            Action::SetRelativeVolume(0.02).to_string(),
            "Increase volume by 2%"
        );
        assert_eq!(
            Action::SetRelativeVolume(-0.02).to_string(),
            "Decrease volume by 2%"
        );
        assert_eq!(
            Action::SetRelativeVolume(0.00).to_string(),
            "Increase volume by 0%"
        );
        assert_eq!(
            Action::SetAbsoluteVolume(0.5).to_string(),
            "Set volume to 50%",
        );
    }

    #[test]
    fn help_empty() {
        let keybindings = HashMap::default();
        let help = Help::from(&keybindings);
        assert!(help.rows.is_empty());
        assert_eq!(help.widths, [0, 0]);
    }

    #[test]
    fn help_single_binding() {
        let mut keybindings = HashMap::new();
        keybindings.insert(
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            Action::Help,
        );

        let help = Help::from(&keybindings);
        assert_eq!(help.rows.len(), 1);
        assert_eq!(
            help.rows[0],
            [String::from("Show/hide help"), String::from("?")]
        );
        assert_eq!(help.widths[0], "Show/hide help".len());
        assert_eq!(help.widths[1], "?".len());
    }

    #[test]
    fn help_nothing_filtered() {
        let mut keybindings = HashMap::new();
        keybindings.insert(
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            Action::Help,
        );
        keybindings.insert(
            KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE),
            Action::Nothing,
        );

        let help = Help::from(&keybindings);
        assert_eq!(help.rows.len(), 1);
        assert_eq!(
            help.rows[0],
            [String::from("Show/hide help"), String::from("?")]
        );
        assert_eq!(help.widths[0], "Show/hide help".len());
        assert_eq!(help.widths[1], "?".len());
    }

    #[test]
    fn help_same_action() {
        let mut keybindings = HashMap::new();
        keybindings.insert(
            KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE),
            Action::Help,
        );
        keybindings.insert(
            KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
            Action::Help,
        );

        let help = Help::from(&keybindings);
        assert_eq!(help.rows.len(), 2);
        // Keybindings should be sorted
        // First binding shows the action name
        assert_eq!(
            help.rows[0],
            [String::from("Show/hide help"), String::from("F1")]
        );
        // Second binding should have empty action name
        assert_eq!(help.rows[1], [String::from(""), String::from("?")]);
        assert_eq!(help.widths[0], "Show/hide help".len());
        assert_eq!(help.widths[1], "F1".len());
    }
}
