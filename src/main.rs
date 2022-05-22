use anyhow::{bail, Context, Error as Anyhow};
use chessboard::engine::Random;
use chessboard::io::{Process, Terminal};
use chessboard::player::{Ai, Cli, Uci};
use chessboard::search::Negamax;
use chessboard::{Color, Game, Outcome, Player, PlayerDispatcher};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use std::{cmp::min, io::stderr};
use tokio::try_join;
use tracing::{info, instrument, warn, Level};
use tracing_subscriber::fmt::format::FmtSpan;
use url::Url;

#[instrument(level = "trace", err)]
async fn player(color: Color, url: Url) -> Result<PlayerDispatcher, Anyhow> {
    if url.has_authority() {
        bail!("urls that have an authority component are not supported");
    }

    if let "ai" = url.scheme() {
        let engine = match url.fragment() {
            Some("random") => Random::default().into(),
            Some(fragment) => bail!("unknown engine '{}'", fragment),
            None => bail!("expected engine as url fragment"),
        };

        let strategy = match url.path() {
            "negamax" => Negamax::new(engine).into(),
            path => bail!("unknown strategy '{}'", path),
        };

        Ok(Ai::new(strategy).into())
    } else {
        let io = match url.path() {
            "" => Terminal::new(color).into(),
            path => Process::spawn(path).await?.into(),
        };

        let player = match url.scheme() {
            "cli" => Cli::new(io).into(),
            "uci" => Uci::init(io).await?.into(),
            scheme => bail!("unknown protocol '{}'", scheme),
        };

        Ok(player)
    }
}

#[instrument(level = "trace", err)]
async fn run(mut white: PlayerDispatcher, mut black: PlayerDispatcher) -> Result<Outcome, Anyhow> {
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
                    .act(position)
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
    #[clap(
        short,
        long,
        value_name = "url",
        default_value = "cli:",
        parse(try_from_str)
    )]
    white: Url,

    /// Black pieces player.
    #[clap(
        short,
        long,
        value_name = "url",
        default_value = "cli:",
        parse(try_from_str)
    )]
    black: Url,

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

    let (white, black) = try_join!(player(Color::White, white), player(Color::Black, black))?;
    let outcome = run(white, black).await?;
    info!(%outcome);

    Ok(())
}
