use anyhow::{bail, Context, Error as Anyhow};
use chessboard::*;
use clap::AppSettings::*;
use future::ok;
use futures::{prelude::*, stream::iter, try_join};
use indicatif::{ProgressBar, ProgressStyle};
use smol::Task;
use std::{borrow::*, cmp::min, collections::*, error::Error, fmt::Debug, io::stderr, num::*};
use structopt::StructOpt;
use tracing::*;
use url::Url;

#[instrument(err)]
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

#[instrument(err)]
#[allow(clippy::unit_arg)]
async fn chessboard<U: Borrow<Url> + Debug>(white: U, black: U) -> Result<Outcome, Anyhow> {
    let mut game = Game::new();

    let (mut white, mut black) = try_join!(
        new_player(Color::White, white.borrow()),
        new_player(Color::Black, black.borrow())
    )?;

    let outcome = loop {
        match game.outcome() {
            Some(o) => break o,

            None => {
                let position = game.position();
                info!(%position);

                let action = match game.player() {
                    Color::Black => black.act(position).await?,
                    Color::White => white.act(position).await?,
                };

                info!(player = %game.player(), %action);

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

    #[structopt(short = "j", long, help = "Runs matches in parallel")]
    parallel: bool,

    #[structopt(
        short,
        long,
        value_name = "level",
        default_value = "info",
        parse(try_from_str)
    )]
    verbosity: Level,

    #[structopt(short, long, help = "Displays progress bar")]
    progress: bool,
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

    let best_of = spec.best_of.get();

    let pb = if spec.progress {
        ProgressBar::new(best_of as u64).with_style(
            ProgressStyle::default_bar()
                .tick_chars("⠉⠙⠹⠸⠼⠴⠤⠦⠧⠇⠏⠋")
                .template("{spinner} [{bar:30}] {pos}/{len} (-{eta})")
                .progress_chars("=> "),
        )
    } else {
        ProgressBar::hidden()
    };

    pb.tick();
    pb.enable_steady_tick(100);

    let matches: Vec<_> = (0..best_of)
        .map(|_| {
            if spec.parallel {
                Task::spawn(chessboard(spec.white.clone(), spec.black.clone())).boxed()
            } else {
                chessboard(&spec.white, &spec.black).boxed()
            }
        })
        .collect();

    let stats = iter(matches)
        .map(Ok)
        .and_then(|o| o)
        .try_fold(BTreeMap::<_, usize>::new(), |mut acc, o| {
            *acc.entry(o.to_string()).or_default() += 1;
            pb.inc(1);
            ok(acc)
        })
        .await
        .context("the match was interrupted")?;

    pb.finish_and_clear();

    let digits = (spec.best_of.get() as f64).log10().ceil() as usize + 1;

    echo!("+{:-<w$}+\n", "", w = digits + 44)?;
    echo!("|{:<w$}|\n", " Statistics ", w = digits + 44)?;
    echo!("+{:-<w$}+\n", "", w = digits + 44)?;
    for (key, abs) in stats {
        let rel = (100 * abs) / best_of;
        echo!("| {:<31} | {:>w$} | {:>3} % |\n", key, abs, rel, w = digits)?;
    }
    echo!("+{:-<w$}+\n", "", w = digits + 44)?;

    Ok(())
}
