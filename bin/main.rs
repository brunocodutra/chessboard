use anyhow::Error as Anyhow;
use clap::Parser;

mod ai;
mod applet;
mod cli;
mod engine;
mod io;

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    cli::Cli::parse().execute().await
}
