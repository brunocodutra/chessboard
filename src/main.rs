use anyhow::Error as Anyhow;
use clap::Parser;

mod cli;

use cli::Cli;

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    Cli::parse().run().await
}
