use anyhow::Error as Anyhow;
use clap::Parser;

mod applet;
mod cli;
mod io;

fn main() -> Result<(), Anyhow> {
    cli::Cli::parse().execute()
}
