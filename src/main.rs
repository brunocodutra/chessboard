use anyhow::{Context, Error as Anyhow};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, io::stderr};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

mod applet;

use applet::{Applet, Execute};

#[derive(Parser)]
#[clap(author, version, about, name = "Chessboard", setting = DeriveDisplayOrder)]
struct Chessboard {
    /// Verbosity level.
    #[clap(short, long, value_name = "level", parse(try_from_str))]
    #[cfg_attr(not(debug_assertions), clap(default_value = "info"))]
    #[cfg_attr(debug_assertions, clap(default_value = "debug"))]
    verbosity: Level,

    #[clap(subcommand)]
    applet: Applet,
}

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    let Chessboard { verbosity, applet } = Parser::parse();

    let filter = format!("{},chessboard={}", min(Level::WARN, verbosity), verbosity);

    tracing_subscriber::fmt()
        .pretty()
        .with_thread_ids(true)
        .with_env_filter(filter)
        .with_writer(stderr)
        .with_span_events(FmtSpan::FULL)
        .try_init()
        .map_err(|e| Anyhow::msg(e.to_string()))
        .context("failed to initialize the tracing infrastructure")?;

    applet.execute().await
}
