use crate::applet::Applet;
use anyhow::Error as Anyhow;
use clap::Parser;
use std::{cmp::min, io::stderr};
use tracing::{instrument, Level};
use tracing_subscriber::fmt::{format::FmtSpan, layer};
use tracing_subscriber::{filter::Targets, prelude::*, registry, util::SubscriberInitExt};

/// Command line interface.
#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    /// Verbosity level.
    #[clap(short, long)]
    #[cfg_attr(not(debug_assertions), clap(default_value_t = Level::INFO))]
    #[cfg_attr(debug_assertions, clap(default_value_t = Level::DEBUG))]
    verbosity: Level,

    #[clap(subcommand)]
    applet: Option<Applet>,
}

impl Cli {
    #[instrument(level = "trace", skip(self), err)]
    pub fn execute(self) -> Result<(), Anyhow> {
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

        self.applet.unwrap_or_default().execute()
    }
}
