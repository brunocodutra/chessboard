use anyhow::Error as Anyhow;
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, io::stderr};
use tracing::Level;
use tracing_subscriber::fmt::{format::FmtSpan, layer};
use tracing_subscriber::{filter::Targets, prelude::*, registry, util::SubscriberInitExt};

mod applet;

use applet::{Applet, Execute};

#[derive(Parser)]
#[clap(author, version, about, name = "Chessboard", setting = DeriveDisplayOrder)]
struct Chessboard {
    /// Verbosity level.
    #[clap(short, long, parse(try_from_str))]
    #[cfg_attr(not(debug_assertions), clap(default_value = "info"))]
    #[cfg_attr(debug_assertions, clap(default_value = "debug"))]
    verbosity: Level,

    #[clap(subcommand)]
    applet: Applet,
}

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    let Chessboard { verbosity, applet } = Parser::parse();

    let filter = Targets::new()
        .with_target("chessboard", verbosity)
        .with_default(min(Level::WARN, verbosity));

    let writer = layer()
        .pretty()
        .with_thread_names(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(stderr);

    registry().with(filter).with(writer).init();

    applet.execute().await
}
