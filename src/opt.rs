//! Parse command-line arguments.

use std::path::PathBuf;

use clap::Parser;

use crate::app::TabKind;
use crate::config;

#[derive(Parser)]
#[clap(name = "wiremix", about = "PipeWire mixer")]
#[command(version = env!("VERGEN_GIT_DESCRIBE"))] // Emitted by build.rs
pub struct Opt {
    #[clap(
        short = 'c',
        long,
        value_name = "FILE",
        help = "Override default config file path"
    )]
    pub config: Option<PathBuf>,

    #[clap(
        short,
        long,
        value_name = "NAME",
        help = "The name of the remote to connect to"
    )]
    pub remote: Option<String>,

    #[clap(
        short,
        long,
        help = "Target frames per second (or 0 for unlimited)"
    )]
    pub fps: Option<f32>,

    #[clap(
        short = 's',
        long,
        value_name = "NAME",
        help = "Character set to use [built-in sets: default, compat, extracompat]"
    )]
    pub char_set: Option<String>,

    #[clap(
        short,
        long,
        value_name = "NAME",
        help = "Theme to use [built-in themes: default, nocolor, plain]"
    )]
    pub theme: Option<String>,

    #[clap(
        short,
        long,
        value_parser = clap::value_parser!(config::Peaks),
        help = "Audio peak meters"
    )]
    pub peaks: Option<config::Peaks>,

    #[clap(long, conflicts_with = "mouse", help = "Disable mouse support")]
    pub no_mouse: bool,

    #[clap(long, conflicts_with = "no_mouse", help = "Enable mouse support")]
    pub mouse: bool,

    #[clap(
        short = 'v',
        long,
        value_enum,
        value_parser = clap::value_parser!(TabKind),
        help = "Initial tab view"
    )]
    pub tab: Option<TabKind>,

    #[cfg(debug_assertions)]
    #[clap(short, long, help = "Dump events without showing interface")]
    pub dump_events: bool,
}

impl Opt {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
