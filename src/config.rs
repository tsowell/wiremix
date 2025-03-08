//! Mixer configuration.

mod name_template;
mod names;
mod tag;

use crossterm::event::KeyCode;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

use crate::app::Action;
use crate::opt::Opt;
use crate::state;

#[derive(Default, Deserialize, Debug)]
pub struct Config {
    pub remote: Option<String>,
    pub fps: Option<f32>,
    #[serde(
        default = "Keybinding::defaults",
        deserialize_with = "Keybinding::merge"
    )]
    pub keybindings: HashMap<KeyCode, Action>,
    #[serde(default)]
    pub names: Names,
}

#[derive(Deserialize, Debug)]
pub struct Keybinding {
    pub key: KeyCode,
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

impl Names {
    fn default_stream() -> Vec<names::NameTemplate> {
        vec!["{node:node.name}: {node:media.name}".parse().unwrap()]
    }

    fn default_endpoint() -> Vec<names::NameTemplate> {
        vec!["{node:node.description}".parse().unwrap()]
    }

    fn default_device() -> Vec<names::NameTemplate> {
        vec!["{device:device.description}".parse().unwrap()]
    }

    /// Tries to resolve an object's name.
    ///
    /// Returns a name using the first template string that can be successfully
    /// resolved using the resolver.
    ///
    /// Precedence is:
    ///
    /// 1. Overrides
    /// 2. Stream/endpoint/device default templates
    /// 3. Fallback
    pub fn resolve<T: names::NameResolver>(
        &self,
        state: &state::State,
        resolver: &T,
    ) -> Option<String> {
        names::resolve(state, resolver, self)
    }
}

impl Default for Names {
    fn default() -> Self {
        Self {
            stream: Self::default_stream(),
            endpoint: Self::default_endpoint(),
            device: Self::default_device(),
            overrides: Default::default(),
        }
    }
}

impl Keybinding {
    fn defaults() -> HashMap<KeyCode, Action> {
        HashMap::from([
            (KeyCode::Char('q'), Action::Exit),
            (KeyCode::Char('m'), Action::ToggleMute),
            (KeyCode::Char('d'), Action::SetDefault),
            (KeyCode::Char('l'), Action::SetRelativeVolume(0.01)),
            (KeyCode::Char('h'), Action::SetRelativeVolume(-0.01)),
            (KeyCode::Char('c'), Action::OpenDropdown),
            (KeyCode::Esc, Action::CloseDropdown),
            (KeyCode::Enter, Action::SelectDropdown),
            (KeyCode::Char('j'), Action::MoveDown),
            (KeyCode::Char('k'), Action::MoveUp),
            (KeyCode::Char('H'), Action::TabLeft),
            (KeyCode::Char('L'), Action::TabRight),
        ])
    }

    /// Merge deserialized keybindings with defaults
    fn merge<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<KeyCode, Action>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut keybindings = Self::defaults();

        let configured = Vec::<Keybinding>::deserialize(deserializer)?;

        for keybinding in configured.into_iter() {
            keybindings.insert(keybinding.key, keybinding.action);
        }

        Ok(keybindings)
    }
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

    /// Override configuration with command-line arguments.
    pub fn apply_opt(&mut self, opt: &Opt) {
        if let Some(remote) = &opt.remote {
            self.remote = Some(remote.clone());
        }

        if let Some(fps) = &opt.fps {
            self.fps = Some(*fps);
        }
    }
}

impl TryFrom<&Path> for Config {
    type Error = anyhow::Error;

    /// Parse configuration from the file at the supplied path.
    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let toml_str = fs::read_to_string(path)?;

        let config: Config = toml::from_str(&toml_str)?;

        Ok(config)
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
