//! Mixer configuration.

mod name_template;
mod names;
mod tag;

use anyhow::Context;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{de::Error, Deserialize};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

use crate::app::Action;
use crate::opt::Opt;

#[derive(Debug)]
pub struct Config {
    pub remote: Option<String>,
    pub fps: Option<f32>,
    pub char_set: CharSet,
    pub keybindings: HashMap<KeyEvent, Action>,
    pub names: Names,
}

/// Represents a configuration deserialized from a file. This gets baked into a
/// Config, which, for example, has a single char_set and theme.
#[derive(Default, Deserialize, Debug)]
struct ConfigFile {
    remote: Option<String>,
    fps: Option<f32>,
    #[serde(default = "default_char_set_name")]
    char_set: String,
    #[serde(
        default = "Keybinding::defaults",
        deserialize_with = "Keybinding::merge"
    )]
    keybindings: HashMap<KeyEvent, Action>,
    #[serde(default)]
    names: Names,
    #[serde(
        default = "CharSet::defaults",
        deserialize_with = "CharSet::merge"
    )]
    char_sets: HashMap<String, CharSet>,
}

#[derive(Deserialize, Debug)]
pub struct Keybinding {
    pub key: KeyCode,
    #[serde(default = "Keybinding::default_modifiers")]
    pub modifiers: KeyModifiers,
    pub action: Action,
}

#[derive(Deserialize, Debug)]
pub struct Names {
    #[serde(default = "Names::default_stream")]
    pub stream: Vec<names::NameTemplate>,
    #[serde(default = "Names::default_endpoint")]
    pub endpoint: Vec<names::NameTemplate>,
    #[serde(default = "Names::default_device")]
    pub device: Vec<names::NameTemplate>,
    #[serde(default)]
    pub overrides: Vec<NameOverride>,
}

#[derive(PartialEq, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OverrideType {
    Stream,
    Endpoint,
    Device,
}

#[derive(Deserialize, Debug)]
pub struct NameOverride {
    pub types: Vec<OverrideType>,
    pub property: names::Tag,
    pub value: String,
    pub templates: Vec<names::NameTemplate>,
}

#[derive(Deserialize, Debug)]
pub struct CharSet {
    /// Default indicator on Input/Output Devices tabs
    pub default_endpoint: String,
    /// Default target on Playback/Recording tabs
    pub default_target: String,
    /// Top character of selected node indicator
    pub object_selected_top: String,
    /// Top character of selected node center
    pub object_selected_center: String,
    /// Top character of selected node bottom
    pub object_selected_bottom: String,
    /// Indicator for more objects off screen
    pub objects_more: String,
    /// Unfilled part of volume bar
    pub volume_bar_background: String,
    /// Filled part of volume bar
    pub volume_bar_foreground: String,
    /// Peak meter left channel - default for _overload and _unlit
    pub meter_left: String,
    /// Peak meter left channel (overload)
    pub meter_left_overload: String,
    /// Peak meter left channel (unlit)
    pub meter_left_unlit: String,
    /// Peak meter right/mono channel - default for _overload and _unlit
    pub meter_right: String,
    /// Peak meter right/mono channel (overload)
    pub meter_right_overload: String,
    /// Peak meter right/mono channel (unlit)
    pub meter_right_unlit: String,
    /// Peak meter left channel live indicator
    pub meter_live_left: String,
    /// Peak meter left channel live indicator (unlit)
    pub meter_live_left_unlit: String,
    /// Peak meter right/mono channel live indicator
    pub meter_live_right: String,
    /// Peak meter right/mono channel live indicator (unlit)
    pub meter_live_right_unlit: String,
    /// Dropdown icon on Configuration tab
    pub dropdown: String,
    /// Selected item in dropdowns
    pub dropdown_item_selected: String,
    /// Indicator for more dropdown items off screen
    pub dropdown_more: String,
    /// Surrounds (left) selected tab in tab menu
    pub tab_selected_left: String,
    /// Surrounds (right) selected tab in tab menu
    pub tab_selected_right: String,
}

// This is what actually gets parsed from the config.
#[derive(Deserialize, Debug)]
struct CharSetOverlay {
    default_endpoint: Option<String>,
    default_target: Option<String>,
    object_selected_top: Option<String>,
    object_selected_center: Option<String>,
    object_selected_bottom: Option<String>,
    objects_more: Option<String>,
    volume_bar_background: Option<String>,
    volume_bar_foreground: Option<String>,
    meter_left: Option<String>,
    meter_left_overload: Option<String>,
    meter_left_unlit: Option<String>,
    meter_right: Option<String>,
    meter_right_overload: Option<String>,
    meter_right_unlit: Option<String>,
    meter_live_left: Option<String>,
    meter_live_left_unlit: Option<String>,
    meter_live_right: Option<String>,
    meter_live_right_unlit: Option<String>,
    dropdown: Option<String>,
    dropdown_item_selected: Option<String>,
    dropdown_more: Option<String>,
    tab_selected_left: Option<String>,
    tab_selected_right: Option<String>,
}

impl Keybinding {
    fn key_event_from(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn defaults() -> HashMap<KeyEvent, Action> {
        HashMap::from([
            (Self::key_event_from(KeyCode::Char('q')), Action::Exit),
            (Self::key_event_from(KeyCode::Char('m')), Action::ToggleMute),
            (Self::key_event_from(KeyCode::Char('d')), Action::SetDefault),
            (
                Self::key_event_from(KeyCode::Char('l')),
                Action::SetRelativeVolume(0.01),
            ),
            (
                Self::key_event_from(KeyCode::Char('h')),
                Action::SetRelativeVolume(-0.01),
            ),
            (
                Self::key_event_from(KeyCode::Char('c')),
                Action::OpenDropdown,
            ),
            (Self::key_event_from(KeyCode::Esc), Action::CloseDropdown),
            (Self::key_event_from(KeyCode::Enter), Action::SelectDropdown),
            (Self::key_event_from(KeyCode::Char('j')), Action::MoveDown),
            (Self::key_event_from(KeyCode::Char('k')), Action::MoveUp),
            (Self::key_event_from(KeyCode::Char('H')), Action::TabLeft),
            (Self::key_event_from(KeyCode::Char('L')), Action::TabRight),
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

    fn default_modifiers() -> KeyModifiers {
        KeyModifiers::NONE
    }

    /// Merge deserialized keybindings with defaults
    fn merge<'de, D>(
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

fn default_char_set_name() -> String {
    String::from("default")
}

impl Default for CharSet {
    fn default() -> Self {
        Self {
            default_endpoint: String::from("◇"),
            default_target: String::from("◇"),
            object_selected_top: String::from("░"),
            object_selected_center: String::from("▒"),
            object_selected_bottom: String::from("░"),
            objects_more: String::from("•••"),
            volume_bar_background: String::from("╌"),
            volume_bar_foreground: String::from("━"),
            meter_left: String::from("▮"),
            meter_left_overload: String::from("▮"),
            meter_left_unlit: String::from("▮"),
            meter_right: String::from("▮"),
            meter_right_overload: String::from("▮"),
            meter_right_unlit: String::from("▮"),
            meter_live_left: String::from("■"),
            meter_live_left_unlit: String::from("■"),
            meter_live_right: String::from("■"),
            meter_live_right_unlit: String::from("■"),
            dropdown: String::from("▼"),
            dropdown_item_selected: String::from(">"),
            dropdown_more: String::from("•••"),
            tab_selected_left: String::from("["),
            tab_selected_right: String::from("]"),
        }
    }
}

impl TryFrom<CharSetOverlay> for CharSet {
    type Error = anyhow::Error;

    fn try_from(overlay: CharSetOverlay) -> Result<Self, Self::Error> {
        let mut char_set: Self = Default::default();

        macro_rules! validate_and_set {
            // Overwrite default char with char from overlay while validating
            // width. Length of 0 means don't check width.
            ($field:ident, $length:expr) => {
                if let Some(value) = overlay.$field {
                    if $length > 0
                        && unicode_width::UnicodeWidthStr::width(value.as_str())
                            != $length
                    {
                        anyhow::bail!(format!(
                            "{} must be {} characters wide",
                            stringify!($field),
                            $length
                        ));
                    }
                    char_set.$field = value;
                }
            };
            ($field:ident, [$($fallback_field:ident),+], $length:expr) => {
                // Do the same, but for multiple fields, using the first one
                // as a default for the fallack fields.
                $(
                    if let (Some(value), None) =
                        (&overlay.$field, &overlay.$fallback_field)
                    {
                        char_set.$fallback_field = value.clone();
                    }
                )+
                validate_and_set!($field, $length);
                $(
                    validate_and_set!($fallback_field, $length);
                )+
            };
        }

        validate_and_set!(default_endpoint, 1);
        validate_and_set!(default_target, 1);
        validate_and_set!(object_selected_top, 1);
        validate_and_set!(object_selected_center, 1);
        validate_and_set!(object_selected_bottom, 1);
        validate_and_set!(objects_more, 0);
        validate_and_set!(volume_bar_background, 1);
        validate_and_set!(volume_bar_foreground, 1);
        validate_and_set!(
            meter_left,
            [meter_left_overload, meter_left_unlit],
            1
        );
        validate_and_set!(
            meter_right,
            [meter_right_overload, meter_right_unlit],
            1
        );
        validate_and_set!(meter_live_left, [meter_live_left_unlit], 1);
        validate_and_set!(meter_live_right, [meter_live_right_unlit], 1);
        validate_and_set!(dropdown, 1);
        validate_and_set!(dropdown_item_selected, 1);
        validate_and_set!(dropdown_more, 0);
        validate_and_set!(tab_selected_left, 1);
        validate_and_set!(tab_selected_right, 1);

        Ok(char_set)
    }
}

impl CharSet {
    fn defaults() -> HashMap<String, CharSet> {
        HashMap::from([(String::from("default"), Default::default())])
    }

    /// Merge deserialized charsets with defaults
    fn merge<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<String, CharSet>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let configured =
            HashMap::<String, CharSetOverlay>::deserialize(deserializer)?;
        let mut merged = configured
            .into_iter()
            .map(|(key, value)| {
                CharSet::try_from(value)
                    .map_err(D::Error::custom)
                    .map(move |charset| (key, charset))
            })
            .collect::<Result<HashMap<String, CharSet>, D::Error>>()?;
        if !merged.contains_key("default") {
            merged.insert(String::from("default"), Default::default());
        }
        Ok(merged)
    }
}

impl ConfigFile {
    /// Override configuration with command-line arguments.
    pub fn apply_opt(&mut self, opt: &Opt) {
        if let Some(remote) = &opt.remote {
            self.remote = Some(remote.clone());
        }

        if let Some(fps) = &opt.fps {
            self.fps = Some(*fps);
        }

        if let Some(char_set) = &opt.char_set {
            self.char_set = char_set.clone();
        }
    }
}

impl TryFrom<ConfigFile> for Config {
    type Error = anyhow::Error;

    fn try_from(mut config_file: ConfigFile) -> Result<Self, Self::Error> {
        let Some(char_set) =
            config_file.char_sets.remove(&config_file.char_set)
        else {
            anyhow::bail!(
                "char_set '{}' does not exist",
                &config_file.char_set
            );
        };

        Ok(Self {
            remote: config_file.remote,
            fps: config_file.fps,
            char_set,
            keybindings: config_file.keybindings,
            names: config_file.names,
        })
    }
}

impl Config {
    /// Returns the configuration file path.
    pub fn default_path() -> Option<PathBuf> {
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return Some(Path::new(&xdg_config).join("pwmixer.conf"));
        }

        if let Ok(home) = env::var("HOME") {
            return Some(Path::new(&home).join(".config/pwmixer.conf"));
        }

        None
    }

    /// Parse configuration from the file at the supplied path.
    pub fn try_new(
        path: Option<&Path>,
        opt: &Opt,
    ) -> Result<Self, anyhow::Error> {
        let mut config_file: ConfigFile = match path {
            Some(path) => {
                let context = || {
                    format!(
                        "Failed to read configuration from file '{}'",
                        path.display()
                    )
                };

                let toml_str =
                    fs::read_to_string(path).with_context(context)?;

                toml::from_str(&toml_str).with_context(context)?
            }
            None => Default::default(),
        };

        config_file.apply_opt(opt);
        let config_file = config_file;

        Self::try_from(config_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_stream() {
        // Just make sure this doesn't panic.
        let _ = Names::default_stream();
    }

    #[test]
    fn test_default_endpoint() {
        // Just make sure this doesn't panic.
        let _ = Names::default_endpoint();
    }

    #[test]
    fn test_default_device() {
        // Just make sure this doesn't panic.
        let _ = Names::default_device();
    }
}
