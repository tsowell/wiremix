#[cfg(feature = "trace")]
use std::path::PathBuf;

use anyhow::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    self, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

pub fn initialize_logging() -> Result<()> {
    let log_file: String = format!("{}.log", env!("CARGO_PKG_NAME"));

    let directory = PathBuf::from(".");
    std::fs::create_dir_all(directory.clone())?;
    let log_path = directory.join(log_file.clone());
    let log_file = std::fs::File::create(log_path)?;
    std::env::set_var("RUST_LOG", std::env::var("RUST_LOG").unwrap());
    let file_subscriber = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::EnvFilter::from_default_env());
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .init();
    Ok(())
}

/// Similar to the `std::dbg!` macro, but generates `tracing` events rather
/// than printing to stdout.
///
/// By default, the verbosity level for the generated events is `DEBUG`, but
/// this can be customized.
#[macro_export]
macro_rules! trace_dbg {
    (target: $target:expr, level: $level:expr, $ex:expr) => {{
        match $ex {
            value => {
                tracing::event!(target: $target, $level, ?value, stringify!($ex));
                value
            }
        }
    }};
    (level: $level:expr, $ex:expr) => {
        trace_dbg!(target: module_path!(), level: $level, $ex)
    };
    (target: $target:expr, $ex:expr) => {
        trace_dbg!(target: $target, level: tracing::Level::DEBUG, $ex)
    };
    ($ex:expr) => {
        trace_dbg!(level: tracing::Level::DEBUG, $ex)
    };
}
