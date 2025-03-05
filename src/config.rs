//! Mixer configuration.

use serde::Deserialize;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

use crate::names;
use crate::opt::Opt;

#[derive(Default, Deserialize, Debug)]
pub struct Config {
    pub remote: Option<String>,
    pub fps: Option<f32>,
    #[serde(default)]
    pub names: Names,
}

#[derive(Deserialize, Debug)]
pub struct Names {
    #[serde(default = "Names::default_stream")]
    pub stream: Vec<String>,
    #[serde(default = "Names::default_endpoint")]
    pub endpoint: Vec<String>,
    #[serde(default = "Names::default_device")]
    pub device: Vec<String>,
    #[serde(default)]
    pub overrides: Vec<NameOverride>,
}

impl Names {
    fn default_stream() -> Vec<String> {
        vec!["{node:node.name}: {node:media.name}".to_string()]
    }

    fn default_endpoint() -> Vec<String> {
        vec!["{node:node.description}".to_string()]
    }

    fn default_device() -> Vec<String> {
        vec!["{device:device.description}".to_string()]
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
    pub property: names::tag::Tag,
    pub value: String,
    pub formats: Vec<String>,
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
