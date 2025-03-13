//! Parse command-line arguments.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[clap(name = "pwmixer", about = "PipeWire mixer")]
pub struct Opt {
    #[clap(
        short,
        long,
        value_name = "NAME",
        help = "The name of the remote to connect to"
    )]
    pub remote: Option<String>,

    // TODO
    #[clap(short, long, help = "Disable audio capture for level monitoring")]
    pub no_capture: bool,

    #[clap(short, long, help = "Target frames per second")]
    pub fps: Option<f32>,

    #[cfg(debug_assertions)]
    #[clap(short, long, help = "Dump events without showing interface")]
    pub dump_events: bool,

    #[clap(
        short = 'c',
        long,
        value_name = "FILE",
        help = "Override default config file path"
    )]
    pub config: Option<PathBuf>,

    #[clap(
        short = 's',
        long,
        value_name = "NAME",
        help = "Character set to use\n[built-in sets: default, compat, extra_compat]"
    )]
    pub char_set: Option<String>,

    #[clap(
        short,
        long,
        value_name = "NAME",
        help = "Theme to use\n[built-in themes: default]"
    )]
    pub theme: Option<String>,
}

impl Opt {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
