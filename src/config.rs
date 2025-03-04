//! Mixer configuration.

use serde::Deserialize;
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

use crate::opt::Opt;

#[derive(Default, Deserialize, Debug)]
pub struct Config {
    pub remote: Option<String>,
    pub fps: Option<f32>,
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
