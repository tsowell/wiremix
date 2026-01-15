//! Parse command-line arguments.

use std::path::PathBuf;

use clap::Parser;

use crate::app::TabKind;
use crate::config;

// VERGEN_GIT_DESCRIBE is emitted by build.rs.
const VERSION: &str = match option_env!("VERGEN_GIT_DESCRIBE") {
    Some(version) => version,
    // VERGEN_GIT_DESCRIBE won't be available when publishing, so fall back to
    // the cargo version.
    None => concat!("v", env!("CARGO_PKG_VERSION")),
};

#[derive(Parser)]
#[clap(name = "wiremix", about = "PipeWire mixer")]
#[command(version = VERSION)]
pub struct Opt {
    /// Override default config file path
    #[clap(short = 'c', long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// The name of the remote to connect to
    #[clap(short, long, value_name = "NAME")]
    pub remote: Option<String>,

    /// Target frames per second (or 0 for unlimited)
    #[clap(short, long)]
    pub fps: Option<f32>,

    /// Character set to use [built-in sets: default, compat, extracompat]
    #[clap(short = 's', long, value_name = "NAME")]
    pub char_set: Option<String>,

    /// Theme to use [built-in themes: default, nocolor, plain]
    #[clap(short, long, value_name = "NAME")]
    pub theme: Option<String>,

    /// Audio peak meters
    #[clap(short, long, value_parser = clap::value_parser!(config::Peaks))]
    pub peaks: Option<config::Peaks>,

    /// Disable mouse support
    #[clap(long, conflicts_with = "mouse")]
    pub no_mouse: bool,

    /// Enable mouse support
    #[clap(long, conflicts_with = "no_mouse")]
    pub mouse: bool,

    /// Initial tab view
    #[clap(
        short = 'v',
        long,
        value_enum,
        value_parser = clap::value_parser!(TabKind),
    )]
    pub tab: Option<TabKind>,

    /// Maximum volume for volume sliders
    #[clap(short = 'm', long, value_name = "PERCENT")]
    pub max_volume_percent: Option<f32>,

    /// Allow increasing volume past max-volume-percent
    #[clap(long, conflicts_with = "enforce_max_volume")]
    pub no_enforce_max_volume: bool,

    /// Prevent increasing volume past max-volume-percent
    #[clap(long, conflicts_with = "no_enforce_max_volume")]
    pub enforce_max_volume: bool,

    #[cfg(debug_assertions)]
    #[clap(short, long)]
    pub dump_events: bool,
}

impl Opt {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
