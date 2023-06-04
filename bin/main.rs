use anyhow::Error as Anyhow;
use clap::Parser;

mod analyze;
mod applet;
mod build;
mod cli;
mod engine;
mod game;
mod io;
mod play;

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    cli::Cli::parse().execute().await
}
