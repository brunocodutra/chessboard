use anyhow::Error as Anyhow;
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, io::stderr};
use tracing::{instrument, Level};
use tracing_subscriber::fmt::{format::FmtSpan, layer};
use tracing_subscriber::{filter::Targets, prelude::*, registry, util::SubscriberInitExt};

mod applet;

use crate::applet::Applet;

/// Command line interface.
#[derive(Parser)]
#[clap(author, version, about, setting = DeriveDisplayOrder)]
pub struct Cli {
    /// Verbosity level.
    #[clap(short, long, parse(try_from_str))]
    #[cfg_attr(not(debug_assertions), clap(default_value = "info"))]
    #[cfg_attr(debug_assertions, clap(default_value = "debug"))]
    verbosity: Level,

    #[clap(subcommand)]
    applet: Option<Applet>,
}

impl Cli {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let filter = Targets::new()
            .with_target("cli", self.verbosity)
            .with_target("lib", self.verbosity)
            .with_default(min(Level::WARN, self.verbosity));

        let writer = layer()
            .pretty()
            .with_thread_names(true)
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(stderr);

        registry().with(filter).with(writer).init();

        self.applet.unwrap_or_default().execute().await
    }
}

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    Cli::parse().execute().await
}
