//! Mixer configuration.

mod char_set;
mod keybinding;
mod name_template;
mod names;
mod tag;
mod theme;

use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Style;
use serde::Deserialize;
use toml;

use crate::app::Action;
use crate::opt::Opt;

#[derive(Debug)]
pub struct Config {
    pub remote: Option<String>,
    pub fps: Option<f32>,
    pub char_set: CharSet,
    pub theme: Theme,
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
    #[serde(default = "default_theme_name")]
    theme: String,
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
    #[serde(default = "Theme::defaults", deserialize_with = "Theme::merge")]
    themes: HashMap<String, Theme>,
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
    pub node_default: String,
    /// Default target on Playback/Recording tabs
    pub target_default: String,
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

#[derive(Deserialize, Debug)]
pub struct Theme {
    pub tab: Style,
    pub tab_selected: Style,
    pub tab_selected_symbols: Style,
    pub object_selected_symbols: Style,
    pub objects_more: Style,
    pub node_name: Style,
    pub node_default_symbol: Style,
    pub volume: Style,
    pub volume_bar_foreground: Style,
    pub volume_bar_background: Style,
    pub meter_unlit: Style,
    pub meter: Style,
    pub meter_overload: Style,
    pub meter_live_unlit: Style,
    pub meter_live: Style,
    pub target: Style,
    pub target_default_symbol: Style,
    pub device_name: Style,
    pub device_dropdown_symbol: Style,
    pub device_profile: Style,
    pub dropdown_border: Style,
    pub dropdown_item: Style,
    pub dropdown_item_selected: Style,
    pub dropdown_more: Style,
}

fn default_char_set_name() -> String {
    String::from("default")
}

fn default_theme_name() -> String {
    String::from("default")
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

        if let Some(theme) = &opt.theme {
            self.theme = theme.clone();
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

        let Some(theme) = config_file.themes.remove(&config_file.theme) else {
            anyhow::bail!("theme '{}' does not exist", &config_file.theme);
        };

        Ok(Self {
            remote: config_file.remote,
            fps: config_file.fps,
            char_set,
            theme,
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
