// SPDX-FileCopyrightText: 2026 Mohamed Hammad <Mohamed.Hammad@SpacecraftSoftware.org>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The machine-readable noun-verb CLI (Spacecraft Software CLI Standard).
//!
//! This runs *alongside* the interactive TUI. `main` dispatches a subcommand
//! here as a one-shot command; with no subcommand it runs the TUI. Everything
//! is built on the existing [`wirehose`](crate::wirehose) layer and the
//! [`View`], so CLI and TUI agree on volume, routing, mute,
//! and defaults.

pub mod dto;
pub mod envelope;
pub mod error;
pub mod oneshot;
pub mod output;
pub mod schema;

use std::io::Write;

use serde::Serialize;
use serde_json::{json, Value};

use crate::config::Config;
use crate::device_kind::DeviceKind;
use crate::view::{Target, View, VolumeAdjustment};
use crate::wirehose::{media_class, ObjectId};

use error::{AppError, ErrorCode, ExitCode};
use oneshot::Connection;
use output::{Mode, Resolved};

/// Tool identity and attribution (Standard §15).
pub const TOOL: &str = "wiremix";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAINTAINER: &str =
    "Mohamed Hammad <Mohamed.Hammad@SpacecraftSoftware.org>";
pub const WEBSITE: &str = "https://Wiremix.SpacecraftSoftware.org/";
pub const DOCS_URL: &str = "https://Wiremix.SpacecraftSoftware.org/";

/// Global flags shared by every subcommand (CLI Standard §3). Flattened into
/// the top-level parser as `global = true` so they may appear before or after
/// the subcommand.
#[derive(clap::Args, Debug, Default, Clone)]
pub struct GlobalArgs {
    /// Machine-readable JSON output (alias for --format json).
    #[clap(long, global = true)]
    pub json: bool,

    /// Output format (explore = interactive TUI).
    #[clap(long, global = true, value_enum, value_name = "FORMAT")]
    pub format: Option<output::Format>,

    /// Restrict output records to these fields (comma-separated).
    #[clap(long, global = true, value_delimiter = ',', value_name = "FIELDS")]
    pub fields: Option<Vec<String>>,

    /// Emit the action plan as JSON and make no changes.
    #[clap(long, global = true)]
    pub dry_run: bool,

    /// Diagnostic output to stderr.
    #[clap(long, global = true)]
    pub verbose: bool,

    /// Suppress non-error stderr.
    #[clap(short = 'q', long, global = true)]
    pub quiet: bool,

    /// When to use color (output is plain text; accepted for compatibility).
    #[clap(long, global = true, value_enum, value_name = "WHEN")]
    pub color: Option<output::ColorWhen>,

    /// Disable color.
    #[clap(long, global = true)]
    pub no_color: bool,

    /// Render absolute time (output is always ISO 8601 UTC regardless).
    #[clap(long, global = true)]
    pub absolute_time: bool,

    /// NUL-terminate output for filename-safe piping.
    #[clap(short = '0', long, global = true)]
    pub print0: bool,

    /// Skip confirmation in non-TTY mode.
    #[clap(long, global = true)]
    pub yes: bool,

    /// Alias for --yes.
    #[clap(long, global = true)]
    pub force: bool,
}

/// The noun-verb command tree.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Inspect and control nodes (streams and device endpoints).
    #[clap(subcommand)]
    Node(NodeCmd),
    /// Inspect and configure devices (profiles).
    #[clap(subcommand)]
    Device(DeviceCmd),
    /// Inspect links between nodes.
    #[clap(subcommand)]
    Link(LinkCmd),
    /// Inspect PipeWire metadata (e.g. defaults).
    #[clap(subcommand)]
    Metadata(MetadataCmd),
    /// Show server/session summary.
    #[clap(subcommand)]
    Server(ServerCmd),
    /// Print the JSON Schema of the CLI.
    Schema,
    /// Print a capability manifest of the CLI.
    Describe,
}

/// How to identify a node: a numeric id, or `--name`.
#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
pub struct NodeTarget {
    /// PipeWire object id (see `node list`).
    #[clap(value_name = "ID")]
    pub id: Option<ObjectId>,
    /// Match by name (case-insensitive substring of name/title), or a token
    /// `@DEFAULT_SINK@` / `@DEFAULT_SOURCE@`.
    #[clap(long, value_name = "NAME")]
    pub name: Option<String>,
}

/// How to identify a device: a numeric id, or `--name`.
#[derive(clap::Args, Debug)]
#[group(required = true, multiple = false)]
pub struct DeviceTarget {
    /// PipeWire object id (see `device list`).
    #[clap(value_name = "ID")]
    pub id: Option<ObjectId>,
    /// Match by name (case-insensitive substring of the device title).
    #[clap(long, value_name = "NAME")]
    pub name: Option<String>,
}

#[derive(clap::Subcommand, Debug)]
pub enum NodeCmd {
    /// List controllable nodes.
    List,
    /// Show one node.
    Get {
        #[clap(flatten)]
        target: NodeTarget,
    },
    /// Set a node's volume. One percentage sets all channels; pass one
    /// percentage per channel to set them independently.
    SetVolume {
        #[clap(flatten)]
        target: NodeTarget,
        #[clap(num_args = 1.., required = true, value_name = "PERCENT")]
        percent: Vec<f32>,
    },
    /// Set a stereo node's balance (-1.0 left .. 0 center .. 1.0 right).
    Balance {
        #[clap(flatten)]
        target: NodeTarget,
        #[clap(value_name = "BALANCE", allow_hyphen_values = true)]
        balance: f32,
    },
    /// Mute, unmute, or toggle a node (default: toggle).
    Mute {
        #[clap(flatten)]
        target: NodeTarget,
        /// Mute the node.
        #[clap(long, conflicts_with_all = ["off", "toggle"])]
        on: bool,
        /// Unmute the node.
        #[clap(long, conflicts_with_all = ["on", "toggle"])]
        off: bool,
        /// Toggle mute (the default).
        #[clap(long, conflicts_with_all = ["on", "off"])]
        toggle: bool,
    },
    /// Set a node as the default sink or source.
    SetDefault {
        #[clap(flatten)]
        target: NodeTarget,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum DeviceCmd {
    /// List devices.
    List,
    /// Show one device.
    Get {
        #[clap(flatten)]
        target: DeviceTarget,
    },
    /// Switch a device to a profile by index.
    SetProfile {
        #[clap(flatten)]
        target: DeviceTarget,
        profile: i32,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum LinkCmd {
    /// List links.
    List,
}

#[derive(clap::Subcommand, Debug)]
pub enum MetadataCmd {
    /// List metadata objects.
    List,
    /// Show one metadata object by name (default: "default").
    Get {
        #[clap(long, default_value = "default")]
        name: String,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ServerCmd {
    /// Show default sink/source, object counts, and remote.
    Info,
}

/// Run a one-shot CLI command and return the process exit code. The PipeWire
/// connection is dropped (cleanly shutting down its thread) before returning.
pub fn run(command: Command, global: &GlobalArgs, config: Config) -> ExitCode {
    let command_str = command_label(&command);

    let resolved = output::resolve(global.json, global.format);
    let mode = match resolved {
        Resolved::Output(mode) => mode,
        // explore + a subcommand is a misuse; report it in a sensible mode.
        Resolved::Tui if global.json => Mode::Json,
        Resolved::Tui => Mode::Human,
    };

    let result = match resolved {
        Resolved::Tui => Err(AppError::usage(
            "`--format explore` is the interactive TUI and cannot be combined \
             with a subcommand",
        )
        .with_hint("wiremix")),
        Resolved::Output(_) => dispatch(command, global, &config, &command_str),
    };

    match result {
        Ok(value) => emit_ok(value, mode, global),
        Err(err) => emit_error(&err, mode, &command_str),
    }
}

fn command_label(command: &Command) -> String {
    let verb = match command {
        Command::Node(c) => match c {
            NodeCmd::List => "node list",
            NodeCmd::Get { .. } => "node get",
            NodeCmd::SetVolume { .. } => "node set-volume",
            NodeCmd::Balance { .. } => "node balance",
            NodeCmd::Mute { .. } => "node mute",
            NodeCmd::SetDefault { .. } => "node set-default",
        },
        Command::Device(c) => match c {
            DeviceCmd::List => "device list",
            DeviceCmd::Get { .. } => "device get",
            DeviceCmd::SetProfile { .. } => "device set-profile",
        },
        Command::Link(LinkCmd::List) => "link list",
        Command::Metadata(c) => match c {
            MetadataCmd::List => "metadata list",
            MetadataCmd::Get { .. } => "metadata get",
        },
        Command::Server(ServerCmd::Info) => "server info",
        Command::Schema => "schema",
        Command::Describe => "describe",
    };
    format!("wiremix {verb}")
}

fn dispatch(
    command: Command,
    global: &GlobalArgs,
    config: &Config,
    command_str: &str,
) -> Result<Value, AppError> {
    match command {
        Command::Schema => Ok(schema::schema()),
        Command::Describe => Ok(schema::describe()),
        Command::Node(cmd) => node(cmd, global, config, command_str),
        Command::Device(cmd) => device(cmd, global, config, command_str),
        Command::Link(LinkCmd::List) => link_list(config, command_str),
        Command::Metadata(cmd) => metadata(cmd, config, command_str),
        Command::Server(ServerCmd::Info) => server_info(config, command_str),
    }
}

// --- Reads & writes: nodes -------------------------------------------------

fn node(
    cmd: NodeCmd,
    global: &GlobalArgs,
    config: &Config,
    command_str: &str,
) -> Result<Value, AppError> {
    match cmd {
        NodeCmd::List => {
            let conn = Connection::open(config.remote.clone())?;
            let view = build_view(&conn, config);
            let nodes: Vec<dto::NodeDto> = view
                .nodes_all
                .iter()
                .filter_map(|id| view.nodes.get(id))
                .map(|node| dto::NodeDto::from(&view, node))
                .collect();
            respond(command_str, nodes, false)
        }
        NodeCmd::Get { target } => {
            let conn = Connection::open(config.remote.clone())?;
            let view = build_view(&conn, config);
            let id = resolve_node(&view, &target)?;
            let node = view.nodes.get(&id).ok_or_else(|| node_not_found(id))?;
            respond(command_str, dto::NodeDto::from(&view, node), false)
        }
        NodeCmd::SetVolume { target, percent } => {
            if global.dry_run {
                return dry_run(
                    command_str,
                    json!({
                        "action": "node set-volume",
                        "target": target_json(&target),
                        "percent": percent,
                    }),
                );
            }
            let mut conn = Connection::open(config.remote.clone())?;
            let id = {
                let view = build_view(&conn, config);
                let id = resolve_node(&view, &target)?;
                set_volume(&conn, &view, config, id, &percent)?;
                id
            };
            conn.settle();
            read_node(&conn, config, id, command_str)
        }
        NodeCmd::Balance { target, balance } => {
            if global.dry_run {
                return dry_run(
                    command_str,
                    json!({
                        "action": "node balance",
                        "target": target_json(&target),
                        "balance": balance,
                    }),
                );
            }
            let mut conn = Connection::open(config.remote.clone())?;
            let id = {
                let view = build_view(&conn, config);
                let id = resolve_node(&view, &target)?;
                set_balance(&conn, &view, id, balance)?;
                id
            };
            conn.settle();
            read_node(&conn, config, id, command_str)
        }
        NodeCmd::Mute {
            target, on, off, ..
        } => {
            if global.dry_run {
                let desired = mute_desired(on, off);
                return dry_run(
                    command_str,
                    json!({
                        "action": "node mute",
                        "target": target_json(&target),
                        "state": desired.map_or("toggle", |m| if m { "on" } else { "off" }),
                    }),
                );
            }
            let mut conn = Connection::open(config.remote.clone())?;
            let id = {
                let view = build_view(&conn, config);
                let id = resolve_node(&view, &target)?;
                let node =
                    view.nodes.get(&id).ok_or_else(|| node_not_found(id))?;
                let should_toggle = match mute_desired(on, off) {
                    Some(desired) => node.mute != desired,
                    None => true,
                };
                if should_toggle {
                    view.mute(id);
                }
                id
            };
            conn.settle();
            read_node(&conn, config, id, command_str)
        }
        NodeCmd::SetDefault { target } => {
            if global.dry_run {
                return dry_run(
                    command_str,
                    json!({
                        "action": "node set-default",
                        "target": target_json(&target),
                    }),
                );
            }
            let mut conn = Connection::open(config.remote.clone())?;
            let id = {
                let view = build_view(&conn, config);
                let id = resolve_node(&view, &target)?;
                let node =
                    view.nodes.get(&id).ok_or_else(|| node_not_found(id))?;
                let kind = if media_class::is_sink(&node.media_class) {
                    DeviceKind::Sink
                } else if media_class::is_source(&node.media_class) {
                    DeviceKind::Source
                } else {
                    return Err(AppError::usage(format!(
                        "node {} is not a sink or source; only devices can be \
                         set as default",
                        u32::from(id)
                    ))
                    .with_hint("wiremix node list --json"));
                };
                view.set_default(id, kind);
                id
            };
            conn.settle();
            read_node(&conn, config, id, command_str)
        }
    }
}

fn read_node(
    conn: &Connection,
    config: &Config,
    id: ObjectId,
    command_str: &str,
) -> Result<Value, AppError> {
    let view = build_view(conn, config);
    let node = view.nodes.get(&id).ok_or_else(|| node_not_found(id))?;
    respond(command_str, dto::NodeDto::from(&view, node), false)
}

// --- Reads & writes: devices ----------------------------------------------

fn device(
    cmd: DeviceCmd,
    global: &GlobalArgs,
    config: &Config,
    command_str: &str,
) -> Result<Value, AppError> {
    match cmd {
        DeviceCmd::List => {
            let conn = Connection::open(config.remote.clone())?;
            let view = build_view(&conn, config);
            let devices: Vec<dto::DeviceDto> = view
                .devices_all
                .iter()
                .filter_map(|id| view.devices.get(id))
                .map(dto::DeviceDto::from)
                .collect();
            respond(command_str, devices, false)
        }
        DeviceCmd::Get { target } => {
            let conn = Connection::open(config.remote.clone())?;
            let view = build_view(&conn, config);
            let id = resolve_device(&view, &target)?;
            let device =
                view.devices.get(&id).ok_or_else(|| device_not_found(id))?;
            respond(command_str, dto::DeviceDto::from(device), false)
        }
        DeviceCmd::SetProfile { target, profile } => {
            if global.dry_run {
                return dry_run(
                    command_str,
                    json!({
                        "action": "device set-profile",
                        "target": device_target_json(&target),
                        "profile": profile,
                    }),
                );
            }
            let mut conn = Connection::open(config.remote.clone())?;
            let id = {
                let view = build_view(&conn, config);
                let id = resolve_device(&view, &target)?;
                let device = view
                    .devices
                    .get(&id)
                    .ok_or_else(|| device_not_found(id))?;
                let known = device.profiles.iter().any(|(target, _)| {
                    matches!(target, Target::Profile(_, index)
                        if *index == profile)
                });
                if !known {
                    return Err(AppError::not_found(format!(
                        "device {} has no profile index {profile}",
                        u32::from(id)
                    ))
                    .with_hint(format!(
                        "wiremix device get {} --json",
                        u32::from(id)
                    )));
                }
                conn.sender().device_set_profile(id, profile);
                id
            };
            conn.settle();
            let view = build_view(&conn, config);
            let device =
                view.devices.get(&id).ok_or_else(|| device_not_found(id))?;
            respond(command_str, dto::DeviceDto::from(device), false)
        }
    }
}

// --- Reads: links, metadata, server ---------------------------------------

fn link_list(config: &Config, command_str: &str) -> Result<Value, AppError> {
    let conn = Connection::open(config.remote.clone())?;
    let links: Vec<dto::LinkDto> = conn
        .state()
        .links
        .iter()
        .map(|(id, link)| dto::LinkDto {
            id: u32::from(*id),
            output_id: link.output_id.into(),
            input_id: link.input_id.into(),
        })
        .collect();
    respond(command_str, links, false)
}

fn metadata(
    cmd: MetadataCmd,
    config: &Config,
    command_str: &str,
) -> Result<Value, AppError> {
    let conn = Connection::open(config.remote.clone())?;
    match cmd {
        MetadataCmd::List => {
            let items: Vec<dto::MetadataDto> = conn
                .state()
                .metadatas
                .values()
                .map(dto::MetadataDto::from)
                .collect();
            respond(command_str, items, false)
        }
        MetadataCmd::Get { name } => {
            let metadata =
                conn.state().get_metadata_by_name(&name).ok_or_else(|| {
                    AppError::not_found(format!("no metadata named {name:?}"))
                        .with_hint("wiremix metadata list --json")
                })?;
            respond(command_str, dto::MetadataDto::from(metadata), false)
        }
    }
}

fn server_info(config: &Config, command_str: &str) -> Result<Value, AppError> {
    let conn = Connection::open(config.remote.clone())?;
    let view = build_view(&conn, config);
    let info =
        dto::ServerInfoDto::from(&view, conn.state(), config.remote.clone());
    respond(command_str, info, true)
}

// --- Helpers ---------------------------------------------------------------

fn build_view<'a>(conn: &'a Connection, config: &Config) -> View<'a> {
    // The CLI shows the full graph: filters are a TUI display preference.
    View::from(conn.sender(), conn.state(), &config.names, &[])
}

/// Resolve a [`NodeTarget`] (id, name substring, or default token) to an id.
fn resolve_node(
    view: &View,
    target: &NodeTarget,
) -> Result<ObjectId, AppError> {
    if let Some(id) = target.id {
        return if view.nodes.contains_key(&id) {
            Ok(id)
        } else {
            Err(node_not_found(id))
        };
    }

    let name = target.name.as_deref().unwrap_or_default();
    match name {
        "@DEFAULT_SINK@" => {
            return view.default_sink.and_then(|t| t.object_id()).ok_or_else(
                || {
                    AppError::not_found("no default sink is set")
                        .with_hint("wiremix server info --json")
                },
            );
        }
        "@DEFAULT_SOURCE@" => {
            return view.default_source.and_then(|t| t.object_id()).ok_or_else(
                || {
                    AppError::not_found("no default source is set")
                        .with_hint("wiremix server info --json")
                },
            );
        }
        _ => {}
    }

    let needle = name.to_lowercase();
    let mut hits = view
        .nodes_all
        .iter()
        .filter_map(|id| view.nodes.get(id))
        .filter(|n| {
            n.name.to_lowercase().contains(&needle)
                || n.title.to_lowercase().contains(&needle)
        })
        .map(|n| n.object_id);
    match (hits.next(), hits.next()) {
        (None, _) => Err(AppError::not_found(format!(
            "no node matches name {name:?}"
        ))
        .with_hint("wiremix node list --json")),
        (Some(id), None) => Ok(id),
        (Some(_), Some(_)) => Err(AppError::new(
            ErrorCode::Conflict,
            format!("multiple nodes match name {name:?}; use an id"),
        )
        .with_hint("wiremix node list --json")),
    }
}

/// Resolve a [`DeviceTarget`] (id or title substring) to an id.
fn resolve_device(
    view: &View,
    target: &DeviceTarget,
) -> Result<ObjectId, AppError> {
    if let Some(id) = target.id {
        return if view.devices.contains_key(&id) {
            Ok(id)
        } else {
            Err(device_not_found(id))
        };
    }

    let name = target.name.as_deref().unwrap_or_default();
    let needle = name.to_lowercase();
    let mut hits = view
        .devices_all
        .iter()
        .filter_map(|id| view.devices.get(id))
        .filter(|d| d.title.to_lowercase().contains(&needle))
        .map(|d| d.object_id);
    match (hits.next(), hits.next()) {
        (None, _) => Err(AppError::not_found(format!(
            "no device matches name {name:?}"
        ))
        .with_hint("wiremix device list --json")),
        (Some(id), None) => Ok(id),
        (Some(_), Some(_)) => Err(AppError::new(
            ErrorCode::Conflict,
            format!("multiple devices match name {name:?}; use an id"),
        )
        .with_hint("wiremix device list --json")),
    }
}

fn target_json(target: &NodeTarget) -> Value {
    match (target.id, &target.name) {
        (Some(id), _) => json!({ "id": u32::from(id) }),
        (None, Some(name)) => json!({ "name": name }),
        (None, None) => Value::Null,
    }
}

fn device_target_json(target: &DeviceTarget) -> Value {
    match (target.id, &target.name) {
        (Some(id), _) => json!({ "id": u32::from(id) }),
        (None, Some(name)) => json!({ "name": name }),
        (None, None) => Value::Null,
    }
}

/// Set a node's volume from one (uniform) or per-channel percentages.
fn set_volume(
    conn: &Connection,
    view: &View,
    config: &Config,
    id: ObjectId,
    percent: &[f32],
) -> Result<(), AppError> {
    let node = view.nodes.get(&id).ok_or_else(|| node_not_found(id))?;
    let channels = node.volumes.len();
    if channels == 0 {
        return Err(AppError::new(
            ErrorCode::Conflict,
            format!("node {} has no controllable channels", u32::from(id)),
        ));
    }

    let max = config
        .enforce_max_volume
        .then_some(config.max_volume_percent);
    if let Some(max) = max {
        if percent.iter().any(|&p| p > max) {
            return Err(AppError::new(
                ErrorCode::Conflict,
                format!("volume exceeds max-volume-percent {max}"),
            )
            .with_hint("pass a lower value or raise max_volume_percent"));
        }
    }

    if percent.len() == 1 {
        // Uniform: reuse the TUI's volume path for exact parity.
        let changed = view.volume(
            id,
            VolumeAdjustment::Absolute(percent[0] / 100.0),
            max,
        );
        if !changed {
            return Err(AppError::new(
                ErrorCode::Conflict,
                String::from("volume change rejected"),
            ));
        }
        return Ok(());
    }

    if percent.len() != channels {
        return Err(AppError::usage(format!(
            "node {} has {channels} channels but {} volumes were given",
            u32::from(id),
            percent.len()
        ))
        .with_hint(format!("wiremix node get {} --json", u32::from(id))));
    }

    let raws = percent
        .iter()
        .map(|&p| (p / 100.0).max(0.0).powi(3))
        .collect();
    issue_channel_volumes(conn, node, raws);
    Ok(())
}

/// Set a stereo node's left/right balance, preserving the loudest channel's
/// level (pavucontrol-style).
fn set_balance(
    conn: &Connection,
    view: &View,
    id: ObjectId,
    balance: f32,
) -> Result<(), AppError> {
    let node = view.nodes.get(&id).ok_or_else(|| node_not_found(id))?;
    if node.volumes.len() != 2 {
        return Err(AppError::usage(format!(
            "node {} is not stereo; balance needs a 2-channel node",
            u32::from(id)
        ))
        .with_hint(format!("wiremix node get {} --json", u32::from(id))));
    }
    let balance = balance.clamp(-1.0, 1.0);
    // Current loudest channel as a 0..1 fraction (cube root of raw amplitude).
    let level = node
        .volumes
        .iter()
        .map(|v| v.cbrt())
        .fold(0.0_f32, f32::max);
    let left = level * if balance > 0.0 { 1.0 - balance } else { 1.0 };
    let right = level * if balance < 0.0 { 1.0 + balance } else { 1.0 };
    let raws = vec![left.max(0.0).powi(3), right.max(0.0).powi(3)];
    issue_channel_volumes(conn, node, raws);
    Ok(())
}

/// Send per-channel volumes, routing to the device or the node directly as the
/// TUI would (based on the node's backing device route).
fn issue_channel_volumes(
    conn: &Connection,
    node: &crate::view::Node,
    raws: Vec<f32>,
) {
    match node.device_info {
        Some((device_id, route_index, route_device)) => {
            conn.sender().device_volumes(
                device_id,
                route_index,
                route_device,
                raws,
            );
        }
        None => conn.sender().node_volumes(node.object_id, raws),
    }
}

fn mute_desired(on: bool, off: bool) -> Option<bool> {
    match (on, off) {
        (true, _) => Some(true),
        (_, true) => Some(false),
        _ => None,
    }
}

fn node_not_found(id: ObjectId) -> AppError {
    AppError::not_found(format!("no node with id {}", u32::from(id)))
        .with_hint("wiremix node list --json")
}

fn device_not_found(id: ObjectId) -> AppError {
    AppError::not_found(format!("no device with id {}", u32::from(id)))
        .with_hint("wiremix device list --json")
}

fn respond<T: Serialize>(
    command: &str,
    data: T,
    attribution: bool,
) -> Result<Value, AppError> {
    let response = envelope::Response::new(command, data);
    let response = if attribution {
        response.with_attribution()
    } else {
        response
    };
    serde_json::to_value(response).map_err(|e| {
        AppError::new(ErrorCode::Internal, format!("serialization failed: {e}"))
    })
}

fn dry_run(command: &str, plan: Value) -> Result<Value, AppError> {
    respond(command, json!({ "dry_run": true, "plan": plan }), false)
}

fn emit_ok(mut value: Value, mode: Mode, global: &GlobalArgs) -> ExitCode {
    if let Some(fields) = &global.fields {
        if let Some(data) = value.get_mut("data") {
            output::project_fields(data, fields);
        }
    }

    let terminator = if global.print0 { '\0' } else { '\n' };
    let mut stdout = std::io::stdout();
    match mode {
        Mode::Json => {
            let _ = write!(stdout, "{}{terminator}", compact(&value));
        }
        Mode::Human => {
            let pretty = serde_json::to_string_pretty(&value)
                .unwrap_or_else(|_| compact(&value));
            let _ = write!(stdout, "{pretty}{terminator}");
        }
        Mode::Jsonl => match value.get("data") {
            Some(Value::Array(items)) => {
                for item in items {
                    let _ = write!(stdout, "{}{terminator}", compact(item));
                }
            }
            Some(other) => {
                let _ = write!(stdout, "{}{terminator}", compact(other));
            }
            None => {
                let _ = write!(stdout, "{}{terminator}", compact(&value));
            }
        },
    }
    let _ = stdout.flush();
    ExitCode::Success
}

fn emit_error(err: &AppError, mode: Mode, command: &str) -> ExitCode {
    match mode {
        Mode::Human => {
            eprintln!("error: {}", err.message);
            if let Some(hint) = &err.hint {
                eprintln!("hint: {hint}");
            }
        }
        Mode::Json | Mode::Jsonl => {
            let object = json!({
                "error": {
                    "code": err.code,
                    "exit_code": err.exit_code().code(),
                    "message": err.message,
                    "hint": err.hint,
                    "timestamp": envelope::now(),
                    "command": command,
                    "docs_url": DOCS_URL,
                }
            });
            eprintln!("{}", compact(&object));
        }
    }
    err.exit_code()
}

fn compact(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("{}"))
}
