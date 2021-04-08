use anyhow::{bail, Context, Error as Anyhow};
use chessboard::{player, remote, Color, Game, Player, PlayerDispatcher, RemoteDispatcher};
use clap::AppSettings::DeriveDisplayOrder;
use futures::try_join;
use smol::block_on;
use std::{cmp::min, error::Error, io};
use structopt::StructOpt;
use tracing::{info, instrument, warn, Level};
use url::Url;

#[instrument(err)]
async fn new_player(color: Color, url: Url) -> Result<PlayerDispatcher<RemoteDispatcher>, Anyhow> {
    let remote = match (url.host_str(), url.path()) {
        (None, "") => remote::Terminal::new(color).into(),
        (Some(host), "") => match url.port() {
            Some(port) => {
                let addr = format!("{}:{}", host, port);
                remote::Tcp::connect(addr).await?.into()
            }
            None => remote::Tcp::connect(host).await?.into(),
        },

        (None, path) => remote::Process::spawn(path.to_string()).await?.into(),
        (Some(_), _) => bail!("remote webservices are not supported yet"),
    };

    let player = match url.scheme() {
        "cli" => player::Cli::new(remote).into(),
        "uci" => player::Uci::init(remote).await?.into(),
        scheme => bail!("unknown protocol '{}'", scheme),
    };

    Ok(player)
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

    #[structopt(
        short,
        long,
        value_name = "level",
        default_value = "info",
        parse(try_from_str)
    )]
    verbosity: Level,
}

#[instrument(err)]
fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let Opts {
        white,
        black,
        verbosity,
    } = Opts::from_args();

    let (writer, _guard) = tracing_appender::non_blocking(io::stderr());
    let filter = format!("{},chessboard={}", min(Level::WARN, verbosity), verbosity);

    tracing_subscriber::fmt()
        .pretty()
        .with_thread_ids(true)
        .with_env_filter(filter)
        .with_writer(writer)
        .try_init()?;

    block_on(async {
        let mut game = Game::default();

        let (mut white, mut black) = try_join!(
            new_player(Color::White, white),
            new_player(Color::Black, black)
        )?;

        let outcome = loop {
            match game.outcome() {
                Some(o) => break o,

                None => {
                    let position = game.position();
                    info!(%position);

                    let action = match position.turn() {
                        Color::Black => black.act(position).await?,
                        Color::White => white.act(position).await?,
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
