use anyhow::Error as Anyhow;
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, io::stderr};
use tracing::Level;
use tracing_subscriber::fmt::{format::FmtSpan, layer};
use tracing_subscriber::{filter::Targets, prelude::*, registry, util::SubscriberInitExt};

mod applet;

use applet::{Applet, Execute};

/// Command line interface.
#[derive(Parser)]
#[clap(author, version, about, name = "Chessboard", setting = DeriveDisplayOrder)]
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
    /// Runs the [`Applet`] requested.
    pub async fn run(self) -> Result<(), Anyhow> {
        let filter = Targets::new()
            .with_target("chessboard", self.verbosity)
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
