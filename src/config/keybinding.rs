//! Implementation for [`Keybinding`](`crate::config::Keybinding`). Defines
//! default bindings and handles merging of configured bindings with defaults.

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

use crate::config::{Action, Keybinding};

impl Keybinding {
    pub fn defaults() -> HashMap<KeyEvent, Action> {
        let event = |code| KeyEvent::new(code, KeyModifiers::NONE);

        HashMap::from([
            (event(KeyCode::Char('q')), Action::Exit),
            (
                // Emulate SIGINT
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                Action::Exit,
            ),
            (
                // Emulate SIGQUIT
                // CrossTerm reports Ctrl-\ as Ctrl-4
                KeyEvent::new(KeyCode::Char('4'), KeyModifiers::CONTROL),
                Action::Exit,
            ),
            (
                // Emulate EOT
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
                Action::Exit,
            ),
            (event(KeyCode::Char('m')), Action::ToggleMute),
            (event(KeyCode::Char('d')), Action::SetDefault),
            (event(KeyCode::Char('l')), Action::SetRelativeVolume(0.01)),
            (event(KeyCode::Right), Action::SetRelativeVolume(0.01)),
            (event(KeyCode::Char('h')), Action::SetRelativeVolume(-0.01)),
            (event(KeyCode::Left), Action::SetRelativeVolume(-0.01)),
            (event(KeyCode::Char('c')), Action::OpenDropdown),
            (event(KeyCode::Esc), Action::CloseDropdown),
            (event(KeyCode::Enter), Action::SelectDropdown),
            (event(KeyCode::Char('j')), Action::MoveDown),
            (event(KeyCode::Down), Action::MoveDown),
            (event(KeyCode::Char('k')), Action::MoveUp),
            (event(KeyCode::Up), Action::MoveUp),
            (event(KeyCode::Char('H')), Action::TabLeft),
            (event(KeyCode::Char('L')), Action::TabRight),
            (
                KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
                Action::TabLeft,
            ),
            (event(KeyCode::Tab), Action::TabRight),
            (event(KeyCode::Char('`')), Action::SetAbsoluteVolume(0.00)),
            (event(KeyCode::Char('1')), Action::SetAbsoluteVolume(0.10)),
            (event(KeyCode::Char('2')), Action::SetAbsoluteVolume(0.20)),
            (event(KeyCode::Char('3')), Action::SetAbsoluteVolume(0.30)),
            (event(KeyCode::Char('4')), Action::SetAbsoluteVolume(0.40)),
            (event(KeyCode::Char('5')), Action::SetAbsoluteVolume(0.50)),
            (event(KeyCode::Char('6')), Action::SetAbsoluteVolume(0.60)),
            (event(KeyCode::Char('7')), Action::SetAbsoluteVolume(0.70)),
            (event(KeyCode::Char('8')), Action::SetAbsoluteVolume(0.80)),
            (event(KeyCode::Char('9')), Action::SetAbsoluteVolume(0.90)),
            (event(KeyCode::Char('0')), Action::SetAbsoluteVolume(1.00)),
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
