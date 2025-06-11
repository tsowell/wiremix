//! Implementation for [`Keybinding`](`crate::config::Keybinding`). Defines
//! default bindings and handles merging of configured bindings with defaults.

use std::collections::HashMap;
use std::os::fd::AsFd;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nix::sys::termios::{self, SpecialCharacterIndices};
use serde::Deserialize;

use crate::config::{Action, Keybinding};

impl Keybinding {
    pub fn defaults() -> HashMap<KeyEvent, Action> {
        let event = |code| KeyEvent::new(code, KeyModifiers::NONE);

        HashMap::from([
            (event(KeyCode::Char('q')), Action::Exit),
            (event(KeyCode::Char('m')), Action::ToggleMute),
            (event(KeyCode::Char('d')), Action::SetDefault),
            (event(KeyCode::Char('l')), Action::SetRelativeVolume(0.01)),
            (event(KeyCode::Right), Action::SetRelativeVolume(0.01)),
            (event(KeyCode::Char('h')), Action::SetRelativeVolume(-0.01)),
            (event(KeyCode::Left), Action::SetRelativeVolume(-0.01)),
            (event(KeyCode::Esc), Action::CloseDropdown),
            (event(KeyCode::Char('c')), Action::ActivateDropdown),
            (event(KeyCode::Enter), Action::ActivateDropdown),
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

    /// Return keybindings emulating effects of certain terminal special
    /// characters
    pub fn control_char_keybindings() -> HashMap<KeyEvent, Action> {
        let mut bindings = HashMap::new();

        let Ok(termios) = termios::tcgetattr(std::io::stdin().as_fd()) else {
            return bindings;
        };

        const SPECIAL_CHAR_INDICES: &[SpecialCharacterIndices] = &[
            SpecialCharacterIndices::VINTR,
            SpecialCharacterIndices::VQUIT,
            SpecialCharacterIndices::VEOF,
        ];

        for &index in SPECIAL_CHAR_INDICES {
            let byte = termios.control_chars[index as usize];

            let key_event = match byte {
                // Handle control characters that are represented by crossterm
                // as non-Char KeyCodes
                9 => KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
                27 => KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                // CrossTerm reports Ctrl-\ as Ctrl-4
                28 => KeyEvent::new(KeyCode::Char('4'), KeyModifiers::CONTROL),

                // Translate the other control characters to control +
                // a printable character
                1..=31 => KeyEvent::new(
                    KeyCode::Char((byte + 96) as char),
                    KeyModifiers::CONTROL,
                ),

                // Pass the printable characters as-is with no modifiers
                32..=126 => KeyEvent::new(
                    KeyCode::Char(byte as char),
                    KeyModifiers::NONE,
                ),
                _ => continue,
            };

            bindings.insert(key_event, Action::Exit);
        }

        bindings
    }
}
