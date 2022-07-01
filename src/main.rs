use anyhow::{Context, Error as Anyhow};
use chessboard::{Game, PlayerConfig, Setup};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, io::stderr};
use tokio::try_join;
use tracing::{info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Parser)]
#[clap(author, version, about, name = "Chessboard", setting = DeriveDisplayOrder)]
struct Args {
    /// White pieces player.
    #[clap(short, long, default_value = "cli()", parse(try_from_str))]
    white: PlayerConfig,

    /// Black pieces player.
    #[clap(short, long, default_value = "cli()", parse(try_from_str))]
    black: PlayerConfig,

    /// Verbosity level.
    #[clap(short, long, value_name = "level", parse(try_from_str))]
    #[cfg_attr(not(debug_assertions), clap(default_value = "info"))]
    #[cfg_attr(debug_assertions, clap(default_value = "debug"))]
    verbosity: Level,
}

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    let Args {
        white,
        black,
        verbosity,
    } = Args::parse();

    let filter = format!("{},chessboard={}", min(Level::WARN, verbosity), verbosity);

    tracing_subscriber::fmt()
        .pretty()
        .with_thread_ids(true)
        .with_env_filter(filter)
        .with_writer(stderr)
        .with_span_events(FmtSpan::FULL)
        .try_init()
        .map_err(|e| Anyhow::msg(e.to_string()))
        .context("failed to initialize the tracing infrastructure")?;

    let (white, black) = try_join!(white.setup(), black.setup())?;

    let mut game = Game::default();
    let report = game.run(white, black).await?;
    info!(outcome = %report.outcome);
    println!("{}", report);

    Ok(())
}
