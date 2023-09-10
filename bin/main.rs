use anyhow::Error as Anyhow;
use clap::Parser;

mod applet;
mod cli;
mod io;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Anyhow> {
    cli::Cli::parse().execute().await
}
