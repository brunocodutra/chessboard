use anyhow::{Context, Error as Anyhow};
use chessboard::{Build, Color, Game, PlayerBuilder};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use libm::erf;
use std::{cmp::min, io::stderr, num::NonZeroUsize};
use tracing::{info, Level};
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Parser)]
#[clap(author, version, about, name = "Chessboard", setting = DeriveDisplayOrder)]
struct Args {
    /// White pieces player.
    #[clap(short, long, value_name = "player", default_value = "cli()")]
    white: PlayerBuilder,

    /// Black pieces player.
    #[clap(short, long, value_name = "player", default_value = "cli()")]
    black: PlayerBuilder,

    /// How many games to play.
    #[clap(short = 'n', long, value_name = "number", default_value = "1")]
    games: NonZeroUsize,

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
        games,
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

    let (mut wins, mut losses, mut draws) = (0f64, 0f64, 0f64);
    let mut reports = Vec::with_capacity(games.into());

    for n in 0..games.into() {
        let white = white.clone().build()?;
        let black = black.clone().build()?;
        let report = Game::default().run(white, black).await?;

        match report.outcome.winner() {
            Some(Color::White) => wins += 1.,
            Some(Color::Black) => losses += 1.,
            None => draws += 1.,
        }

        info!(
            games = n + 1,
            white = wins + draws / 2.,
            black = losses + draws / 2.,
            ΔELO = -400. * ((wins + losses + draws) / (wins + draws / 2.) - 1.).log10(),
            LOS = (1. + erf((wins - losses) / (2. * (wins + losses)).sqrt())) / 2.
        );

        reports.push(report);
    }

    for report in reports {
        println!("{}\n", report);
    }

    Ok(())
}
