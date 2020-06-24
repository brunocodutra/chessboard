use anyhow::{bail, Context, Error as Anyhow};
use chessboard::*;
use clap::AppSettings::*;
use futures::{prelude::*, stream::iter, try_join};
use std::{cmp::min, collections::BTreeMap, error::Error, io::stderr, num::NonZeroUsize};
use structopt::StructOpt;
use tracing::*;
use url::Url;

async fn new_player(color: Color, url: &Url) -> Result<ActorDispatcher<RemoteDispatcher>, Anyhow> {
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

    let actor = match url.scheme() {
        "cli" => actor::Cli::new(remote).into(),
        "uci" => actor::Uci::init(remote).await?.into(),
        scheme => bail!("unknown protocol '{}'", scheme),
    };

    Ok(actor)
}

#[instrument(skip(white, black), err)]
#[allow(clippy::unit_arg)]
async fn chessboard(white: &Url, black: &Url) -> Result<Outcome, Anyhow> {
    let mut game = Game::new();

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

                let action = match game.player().color {
                    Color::Black => black.act(position).await?,
                    Color::White => white.act(position).await?,
                };

                info!(player = %game.player().color, %action);

                if let Err(e) = game.execute(action).context("invalid player action") {
                    warn!("{:?}", e);
                }
            }
        }
    };

    info!(%outcome);

    Ok(outcome)
}

#[derive(StructOpt)]
#[structopt(author, name = "Chessboard", setting = DeriveDisplayOrder)]
struct AppSpec {
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
        short = "n",
        long,
        value_name = "n",
        default_value = "1",
        parse(try_from_str)
    )]
    best_of: NonZeroUsize,

    #[structopt(
        short,
        long,
        value_name = "level",
        default_value = "info",
        parse(try_from_str)
    )]
    verbosity: Level,
}

macro_rules! echo {
    ($($arg:tt)*) => ({
        async_std::io::stdout().write_all(format!($($arg)*).as_bytes()).await
    })
}

#[smol_potat::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let spec = AppSpec::from_args();

    let (writer, _guard) = tracing_appender::non_blocking(stderr());

    let filter = format!(
        "{},chessboard={}",
        min(Level::WARN, spec.verbosity.clone()),
        spec.verbosity
    );

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_env_filter(filter)
        .try_init()?;

    let stats = iter(0..spec.best_of.get())
        .map(Ok)
        .and_then(|_| chessboard(&spec.white, &spec.black))
        .try_fold(BTreeMap::<_, usize>::new(), |mut acc, o| async move {
            *acc.entry(o.to_string()).or_default() += 1;
            Ok(acc)
        })
        .await
        .context("the match was interrupted")?;

    let width = (spec.best_of.get() as f64).log10().ceil() as usize + 1;

    echo!("+{:-<w$}+\n", "", w = width + 44)?;
    echo!("| {:<w$} |\n", "Statistics", w = width + 42)?;
    echo!("+{:-<w$}+\n", "", w = width + 44)?;
    for (key, abs) in stats {
        let rel = (100 * abs) / spec.best_of.get();
        echo!("| {:<31} | {:>w$} | {:>3} % |\n", key, abs, rel, w = width)?;
    }
    echo!("+{:-<w$}+\n", "", w = width + 44)?;

    Ok(())
}
