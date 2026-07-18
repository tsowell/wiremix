// SPDX-FileCopyrightText: 2026 Mohamed Hammad <Mohamed.Hammad@SpacecraftSoftware.org>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Output-mode detection and field projection (CLI Standard §5, §6, §8).

use std::collections::BTreeSet;
use std::io::IsTerminal;

use serde_json::Value;

/// The `--format` values accepted by every command.
///
/// `yaml` and `csv` from the full set defined by the CLI Standard are not yet implemented; `json` and
/// `jsonl` cover the machine-readable contract, `explore` is the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Format {
    Json,
    Jsonl,
    Explore,
}

/// `--color` control. Output is plain text, so this is accepted for
/// cross-CLI compatibility but currently has no visible effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum ColorWhen {
    Never,
    Always,
    Auto,
}

/// The resolved rendering for a one-shot command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Human,
    Json,
    Jsonl,
}

/// Outcome of the §5 detection cascade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    /// Run the interactive TUI (`--format explore`).
    Tui,
    /// Produce one-shot output in the given mode.
    Output(Mode),
}

/// Resolve the effective output mode (CLI Standard §5 cascade). First match
/// wins: explicit flag → agent env → stdout TTY → non-TTY → human fallback.
pub fn resolve(json: bool, format: Option<Format>) -> Resolved {
    if json {
        return Resolved::Output(Mode::Json);
    }
    if let Some(format) = format {
        return match format {
            Format::Json => Resolved::Output(Mode::Json),
            Format::Jsonl => Resolved::Output(Mode::Jsonl),
            Format::Explore => Resolved::Tui,
        };
    }
    if agent_env() {
        return Resolved::Output(Mode::Json);
    }
    if std::io::stdout().is_terminal() {
        Resolved::Output(Mode::Human)
    } else {
        Resolved::Output(Mode::Json)
    }
}

/// Whether an agent/automation environment variable requests machine mode
/// (CLI Standard §5, Agentic CLI §4). `CLAUDECODE`/`CURSOR_AGENT`/`GEMINI_CLI`
/// are informational only and deliberately not consulted here.
pub fn agent_env() -> bool {
    env_true("AI_AGENT") || env_true("AGENT") || env_true("CI")
}

fn env_true(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|value| {
        !value.is_empty() && value != "0" && value != "false"
    })
}

/// Restrict every record in `value` to the requested field names
/// (`--fields`). Reduces token cost for agents (§8). Non-object values and
/// unknown field names are left untouched / dropped respectively.
pub fn project_fields(value: &mut Value, fields: &[String]) {
    let keep: BTreeSet<&str> = fields.iter().map(String::as_str).collect();
    match value {
        Value::Array(items) => {
            for item in items.iter_mut() {
                retain(item, &keep);
            }
        }
        other => retain(other, &keep),
    }
}

fn retain(value: &mut Value, keep: &BTreeSet<&str>) {
    if let Value::Object(map) = value {
        map.retain(|key, _| keep.contains(key.as_str()));
    }
}
