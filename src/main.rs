use anyhow::{Context, Error as Anyhow};
use chessboard::{Color, Game, Outcome, Play, PlayerConfig, Setup};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, error::Error, fmt::Debug, io::stderr};
use tokio::try_join;
use tracing::{info, instrument, warn, Level};
use tracing_subscriber::fmt::format::FmtSpan;

#[instrument(level = "trace", err)]
async fn run<T>(mut white: T, mut black: T) -> Result<Outcome, Anyhow>
where
    T: Play + Debug,
    T::Error: Error + Send + Sync + 'static,
{
    let mut game = Game::default();

    loop {
        match game.outcome() {
            Some(o) => break Ok(o),

            None => {
                let position = game.position();
                info!(%position);

                let turn = position.turn();

                let player = match turn {
                    Color::Black => &mut black,
                    Color::White => &mut white,
                };

                let action = player
                    .play(position)
                    .await
                    .context(format!("the {} player encountered an error", turn))?;

                info!(player = %turn, %action);

                if let Err(e) = game.execute(action).context("invalid player action") {
                    warn!("{:?}", e);
                }
            }
        }
    }
}

#[derive(Parser)]
#[clap(author, version, about, name = "Chessboard", setting = DeriveDisplayOrder)]
struct Opts {
    /// White pieces player.
    #[clap(short, long, default_value = "cli(term)", parse(try_from_str))]
    white: PlayerConfig,

    /// Black pieces player.
    #[clap(short, long, default_value = "cli(term)", parse(try_from_str))]
    black: PlayerConfig,

    /// Verbosity level.
    #[clap(short, long, value_name = "level", parse(try_from_str))]
    #[cfg_attr(not(debug_assertions), clap(default_value = "info"))]
    #[cfg_attr(debug_assertions, clap(default_value = "debug"))]
    verbosity: Level,
}

#[tokio::main]
async fn main() -> Result<(), Anyhow> {
    let Opts {
        white,
        black,
        verbosity,
    } = Opts::parse();

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
    let outcome = run(white, black).await?;
    info!(%outcome);

    Ok(())
}
