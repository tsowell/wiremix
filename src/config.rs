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
use ratatui::{style::Style, widgets::block::BorderType};
use serde::Deserialize;
use toml;

use crate::app::Action;
use crate::opt::Opt;

#[derive(Debug)]
pub struct Config {
    pub remote: Option<String>,
    pub fps: Option<f32>,
    pub mouse: bool,
    pub peaks: Peaks,
    pub char_set: CharSet,
    pub theme: Theme,
    pub keybindings: HashMap<KeyEvent, Action>,
    pub names: Names,
}

/// Represents a configuration deserialized from a file. This gets baked into a
/// Config, which, for example, has a single char_set and theme.
#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    remote: Option<String>,
    fps: Option<f32>,
    #[serde(default = "default_mouse")]
    mouse: bool,
    peaks: Option<Peaks>,
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

// The serde defaults need to be repeated here, which is used to generate a
// default ConfigFile when there is no config file to parse.
impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            remote: Default::default(),
            fps: Default::default(),
            mouse: default_mouse(),
            peaks: Default::default(),
            char_set: default_char_set_name(),
            theme: default_theme_name(),
            keybindings: Keybinding::defaults(),
            names: Default::default(),
            char_sets: CharSet::defaults(),
            themes: Theme::defaults(),
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Peaks {
    Off,
    Mono,
    #[default]
    Auto,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Keybinding {
    pub key: KeyCode,
    #[serde(default = "Keybinding::default_modifiers")]
    pub modifiers: KeyModifiers,
    pub action: Action,
}

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
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
#[cfg_attr(test, derive(PartialEq))]
#[serde(deny_unknown_fields)]
pub struct NameOverride {
    pub types: Vec<OverrideType>,
    pub property: names::Tag,
    pub value: String,
    pub templates: Vec<names::NameTemplate>,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct CharSet {
    pub default_device: String,
    pub default_stream: String,
    pub selector_top: String,
    pub selector_middle: String,
    pub selector_bottom: String,
    pub tab_marker_left: String,
    pub tab_marker_right: String,
    pub list_more: String,
    pub volume_empty: String,
    pub volume_filled: String,
    pub meter_left_inactive: String,
    pub meter_left_active: String,
    pub meter_left_overload: String,
    pub meter_right_inactive: String,
    pub meter_right_active: String,
    pub meter_right_overload: String,
    pub meter_center_left_inactive: String,
    pub meter_center_left_active: String,
    pub meter_center_right_inactive: String,
    pub meter_center_right_active: String,
    pub dropdown_icon: String,
    pub dropdown_selector: String,
    pub dropdown_more: String,
    pub dropdown_border: BorderType,
}

#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Theme {
    pub default_device: Style,
    pub default_stream: Style,
    pub selector: Style,
    pub tab: Style,
    pub tab_selected: Style,
    pub tab_marker: Style,
    pub list_more: Style,
    pub node_title: Style,
    pub node_target: Style,
    pub volume: Style,
    pub volume_empty: Style,
    pub volume_filled: Style,
    pub meter_inactive: Style,
    pub meter_active: Style,
    pub meter_overload: Style,
    pub meter_center_inactive: Style,
    pub meter_center_active: Style,
    pub config_device: Style,
    pub config_profile: Style,
    pub dropdown_icon: Style,
    pub dropdown_border: Style,
    pub dropdown_item: Style,
    pub dropdown_selected: Style,
    pub dropdown_more: Style,
}

fn default_mouse() -> bool {
    true
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

        if let Some(fps) = opt.fps {
            self.fps = (fps != 0.0).then_some(fps);
        }

        if opt.no_mouse {
            self.mouse = false;
        }

        if opt.mouse {
            self.mouse = true;
        }

        if let Some(peaks) = &opt.peaks {
            self.peaks = Some(peaks.clone());
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
            mouse: config_file.mouse,
            peaks: config_file.peaks.unwrap_or_default(),
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
            return Some(Path::new(&xdg_config).join("wiremix/wiremix.toml"));
        }

        if let Ok(home) = env::var("HOME") {
            return Some(Path::new(&home).join(".config/wiremix/wiremix.toml"));
        }

        None
    }

    /// Parse configuration from the file at the supplied path.
    pub fn try_new(
        path: Option<&Path>,
        opt: &Opt,
    ) -> Result<Self, anyhow::Error> {
        let mut config_file: ConfigFile = match path {
            Some(path) if path.exists() => {
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
            _ => ConfigFile::default(),
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
    fn empty_config_matches_default() {
        let empty_config: ConfigFile = toml::from_str("").unwrap();

        assert_eq!(empty_config, ConfigFile::default());
    }

    #[test]
    fn unknown_field_config_file() {
        let config = r#"
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<ConfigFile>(&config).is_err());
    }

    #[test]
    fn unknown_field_keybinding() {
        let config = r#"
        key = { Char = "x" }
        action = "Nothing"
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<Keybinding>(&config).is_err());
    }

    #[test]
    fn unknown_field_names() {
        let config = r#"
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<Names>(&config).is_err());
    }

    #[test]
    fn unknown_field_name_override() {
        let config = r#"
        types = [ "stream" ]
        property = "node:node.name"
        value = "value"
        templates = [ "template" ]
        unknown = "unknown"
        "#;
        assert!(toml::from_str::<NameOverride>(&config).is_err());
    }
}
