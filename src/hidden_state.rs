//! Durable, cross-restart persistence for permanently-hidden items.
//!
//! Items are identified by [`MatchCondition`] (built via
//! [`MatchCondition::from_node_name`]) rather than raw object IDs, since
//! object IDs never survive a restart. This is the same matching engine
//! [`[[filters]]`](`crate::config::Filter`) already uses.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::config::MatchCondition;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct HiddenState {
    #[serde(default)]
    pub hidden: Vec<MatchCondition>,
}

impl HiddenState {
    /// Bare filename (no directory) the state file is saved under - shared
    /// with the inotify-based live-sync watch (see
    /// `wirehose::hidden_state_watch`), which needs to filter directory
    /// events down to just this file without duplicating the string.
    pub const FILENAME: &'static str = "hidden.toml";

    /// Returns the default path for the hidden-item state file, following
    /// the same `$XDG_STATE_HOME`/`$HOME` resolution
    /// [`Config::default_path`](`crate::config::Config::default_path`)
    /// already uses for the config file (mirroring `$XDG_CONFIG_HOME`).
    pub fn default_path() -> Option<PathBuf> {
        if let Ok(xdg_state) = env::var("XDG_STATE_HOME") {
            return Some(
                Path::new(&xdg_state).join("wiremix").join(Self::FILENAME),
            );
        }

        if let Ok(home) = env::var("HOME") {
            return Some(
                Path::new(&home)
                    .join(".local/state/wiremix")
                    .join(Self::FILENAME),
            );
        }

        None
    }

    /// Loads persisted hidden-item matchers from `path`. A missing file is
    /// the normal case before anything has ever been permanently hidden,
    /// not an error - returns an empty state instead.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let context = || {
            format!(
                "Failed to read hidden-item state from file '{}'",
                path.display()
            )
        };

        let toml_str = fs::read_to_string(path).with_context(context)?;
        toml::from_str(&toml_str).with_context(context)
    }

    /// Saves to `path`, creating its parent directory if needed. Writes to
    /// a temp file in the same directory first and renames it into place,
    /// so a crash - or another wiremix instance saving its own state at
    /// the same moment - can never leave a half-written file behind. Two
    /// instances racing to save at the same moment still means whichever
    /// finishes last simply overwrites the other (no cross-process
    /// locking), the same best-effort tradeoff already documented for
    /// live metadata sync.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create directory '{}'", parent.display())
            })?;
        }

        let toml_str = toml::to_string_pretty(self)
            .context("Failed to serialize hidden-item state")?;

        let tmp_path = path.with_extension("toml.tmp");
        fs::write(&tmp_path, toml_str).with_context(|| {
            format!("Failed to write '{}'", tmp_path.display())
        })?;
        fs::rename(&tmp_path, path).with_context(|| {
            format!(
                "Failed to rename '{}' to '{}'",
                tmp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_file_returns_empty() {
        let state =
            HiddenState::load(Path::new("/nonexistent/path/hidden.toml"))
                .unwrap();
        assert!(state.hidden.is_empty());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = std::env::temp_dir()
            .join(format!("wiremix-hidden-state-test-{}", std::process::id()));
        let path = dir.join("hidden.toml");

        let state = HiddenState {
            hidden: vec![MatchCondition::from_node_name("test-node")],
        };
        state.save(&path).unwrap();

        let loaded = HiddenState::load(&path).unwrap();
        assert_eq!(loaded.hidden.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
