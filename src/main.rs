use anyhow::{bail, Context, Error as Anyhow};
use chessboard::player::{Ai, Cli, Uci};
use chessboard::remote::{Process, Tcp, Terminal};
use chessboard::search::Random;
use chessboard::{Color, Game, Player, PlayerDispatcher};
use clap::AppSettings::DeriveDisplayOrder;
use futures::try_join;
use smol::block_on;
use std::io::{stderr, BufWriter};
use std::{cmp::min, error::Error};
use structopt::StructOpt;
use tracing::{info, instrument, warn, Level};
use tracing_subscriber::fmt::format::FmtSpan;
use url::Url;

#[instrument(level = "trace", err)]
async fn player(color: Color, url: Url) -> Result<PlayerDispatcher, Anyhow> {
    if let "ai" = url.scheme() {
        let strategy = match url.path() {
            "random" => Random::default().into(),
            search => bail!("unsupported strategy '{}'", search),
        };

        Ok(Ai::new(strategy).into())
    } else {
        let remote = match (url.host_str(), url.path()) {
            (None, "") => Terminal::new(color).into(),
            (None, path) => Process::spawn(path.to_string()).await?.into(),

            (Some(host), "") => match url.port() {
                Some(port) => Tcp::connect(format!("{}:{}", host, port)).await?.into(),
                None => Tcp::connect(host).await?.into(),
            },

            _ => bail!("remote webservices are not supported yet"),
        };

        let player = match url.scheme() {
            "cli" => Cli::new(remote).into(),
            "uci" => Uci::init(remote).await?.into(),
            scheme => bail!("unsupported protocol '{}'", scheme),
        };

        Ok(player)
    }
}

#[derive(StructOpt)]
#[structopt(author, name = "Chessboard", setting = DeriveDisplayOrder)]
struct Opts {
    #[structopt(
        short,
        long,
        value_name = "url",
        default_value = "cli:",
        parse(try_from_str)
    )]
    white: Url,

    #[structopt(
        short,
        long,
        value_name = "url",
        default_value = "cli:",
        parse(try_from_str)
    )]
    black: Url,

    #[structopt(short, long, value_name = "level", parse(try_from_str))]
    #[cfg_attr(not(debug_assertions), structopt(default_value = "info"))]
    #[cfg_attr(debug_assertions, structopt(default_value = "debug"))]
    verbosity: Level,
}

#[instrument(level = "trace", err)]
fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let Opts {
        white,
        black,
        verbosity,
    } = Opts::from_args();

    let filter = format!("{},chessboard={}", min(Level::WARN, verbosity), verbosity);

    tracing_subscriber::fmt()
        .pretty()
        .with_thread_ids(true)
        .with_env_filter(filter)
        .with_writer(|| BufWriter::new(stderr()))
        .with_span_events(FmtSpan::FULL)
        .try_init()?;

    block_on(async {
        use Color::*;

        let mut game = Game::default();
        let (mut white, mut black) = try_join!(player(White, white), player(Black, black))?;

        let outcome = loop {
            match game.outcome() {
                Some(o) => break o,

                None => {
                    let position = game.position();
                    info!(%position);

                    let action = match position.turn() {
                        Black => black.act(position).await?,
                        White => white.act(position).await?,
                    };

                    info!(player = %position.turn(), ?action);

                    if let Err(e) = game.execute(action).context("invalid player action") {
                        warn!("{:?}", e);
                    }
                }
            }
        };

        info!(%outcome);

        Ok(())
    })
}
