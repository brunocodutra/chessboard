use crate::applet::Applet;
use anyhow::Error as Anyhow;
use clap::Parser;

/// Command line interface.
#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[clap(subcommand)]
    applet: Option<Applet>,
}

impl Cli {
    pub fn execute(self) -> Result<(), Anyhow> {
        self.applet.unwrap_or_default().execute()
    }
}
