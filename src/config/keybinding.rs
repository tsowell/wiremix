//! Implementation for [`Keybinding`](`crate::config::Keybinding`). Defines
//! default bindings and handles merging of configured bindings with defaults.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::config::{Action, Keybinding};

impl Keybinding {
    fn key_event_from(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    pub fn defaults() -> HashMap<KeyEvent, Action> {
        HashMap::from([
            (Self::key_event_from(KeyCode::Char('q')), Action::Exit),
            (Self::key_event_from(KeyCode::Char('m')), Action::ToggleMute),
            (Self::key_event_from(KeyCode::Char('d')), Action::SetDefault),
            (
                Self::key_event_from(KeyCode::Char('l')),
                Action::SetRelativeVolume(0.01),
            ),
            (
                Self::key_event_from(KeyCode::Right),
                Action::SetRelativeVolume(0.01),
            ),
            (
                Self::key_event_from(KeyCode::Char('h')),
                Action::SetRelativeVolume(-0.01),
            ),
            (
                Self::key_event_from(KeyCode::Left),
                Action::SetRelativeVolume(-0.01),
            ),
            (
                Self::key_event_from(KeyCode::Char('c')),
                Action::OpenDropdown,
            ),
            (Self::key_event_from(KeyCode::Esc), Action::CloseDropdown),
            (Self::key_event_from(KeyCode::Enter), Action::SelectDropdown),
            (Self::key_event_from(KeyCode::Char('j')), Action::MoveDown),
            (Self::key_event_from(KeyCode::Down), Action::MoveDown),
            (Self::key_event_from(KeyCode::Char('k')), Action::MoveUp),
            (Self::key_event_from(KeyCode::Up), Action::MoveUp),
            (Self::key_event_from(KeyCode::Char('H')), Action::TabLeft),
            (Self::key_event_from(KeyCode::Char('L')), Action::TabRight),
            (Self::key_event_from(KeyCode::Tab), Action::TabRight),
            (
                Self::key_event_from(KeyCode::Char('`')),
                Action::SetAbsoluteVolume(0.00),
            ),
            (
                Self::key_event_from(KeyCode::Char('1')),
                Action::SetAbsoluteVolume(0.10),
            ),
            (
                Self::key_event_from(KeyCode::Char('2')),
                Action::SetAbsoluteVolume(0.20),
            ),
            (
                Self::key_event_from(KeyCode::Char('3')),
                Action::SetAbsoluteVolume(0.30),
            ),
            (
                Self::key_event_from(KeyCode::Char('4')),
                Action::SetAbsoluteVolume(0.40),
            ),
            (
                Self::key_event_from(KeyCode::Char('5')),
                Action::SetAbsoluteVolume(0.50),
            ),
            (
                Self::key_event_from(KeyCode::Char('6')),
                Action::SetAbsoluteVolume(0.60),
            ),
            (
                Self::key_event_from(KeyCode::Char('7')),
                Action::SetAbsoluteVolume(0.70),
            ),
            (
                Self::key_event_from(KeyCode::Char('8')),
                Action::SetAbsoluteVolume(0.80),
            ),
            (
                Self::key_event_from(KeyCode::Char('9')),
                Action::SetAbsoluteVolume(0.90),
            ),
            (
                Self::key_event_from(KeyCode::Char('0')),
                Action::SetAbsoluteVolume(1.00),
            ),
        ])
    }

    pub fn default_modifiers() -> KeyModifiers {
        KeyModifiers::NONE
    }

    /// Merge deserialized keybindings with defaults
    pub fn merge<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<KeyEvent, Action>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut keybindings = Self::defaults();

        let configured = Vec::<Keybinding>::deserialize(deserializer)?;

        for keybinding in configured.into_iter() {
            keybindings.insert(
                KeyEvent::new(keybinding.key, keybinding.modifiers),
                keybinding.action,
            );
        }

        Ok(keybindings)
    }
}
