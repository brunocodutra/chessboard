use crate::{engine::EngineConfig, game::Game};
use anyhow::Error as Anyhow;
use clap::Parser;
use derive_more::{DebugCustom, Display};
use futures_util::future::try_join_all;
use lib::chess::{Color, Position};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{iter::repeat, num::NonZeroUsize, sync::Arc};
use tokio::{spawn, sync::Mutex};
use tracing::{info, instrument};

/// A match of chess between two players.
#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Play {
    /// Total number of games to play.
    #[clap(short = 'n', long, default_value = "1")]
    games: NonZeroUsize,

    /// Maximum number of games to play in parallel.
    #[clap(short = 'c', long, default_value = "1")]
    concurrency: NonZeroUsize,

    /// The challenging player.
    challenger: EngineConfig,

    /// The defending player.
    defender: EngineConfig,
}

impl Play {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let players = [self.challenger, self.defender];
        let games = Arc::new(AtomicUsize::new(self.games.into()));
        let wld = Arc::new(Mutex::new([0; 3]));

        let tasks = repeat((players, games, wld))
            .take(self.concurrency.into())
            .map(move |(players, games, wld)| spawn(play(players, games, wld)));

        try_join_all(tasks).await?;

        Ok(())
    }
}

#[derive(DebugCustom, Display, Copy, Clone)]
#[debug(fmt = "{self}")]
#[display(fmt = "[{_0:+.2}, {_1:+.2}]")]
struct ΔELO(f64, f64);

impl ΔELO {
    fn new([wins, losses, draws]: [usize; 3]) -> Self {
        let n = wins as f64 + losses as f64 + draws as f64;
        let w = wins as f64 / n;
        let l = losses as f64 / n;
        let d = draws as f64 / n;

        let wvar = w * (l + d / 2.).powi(2);
        let lvar = l * (w + d / 2.).powi(2);
        let dvar = d * (w - l).powi(2);

        let error = (wvar + lvar + dvar).sqrt() / n.sqrt();

        ΔELO(
            400. * ((w - 1.96 * error).max(0.) / (l + 1.96 * error)).log10(),
            400. * ((w + 1.96 * error) / (l - 1.96 * error).max(0.)).log10(),
        )
    }
}

#[instrument(level = "trace", skip(players, games, wld), err)]
pub async fn play(
    players: [EngineConfig; 2],
    games: Arc<AtomicUsize>,
    wld: Arc<Mutex<[usize; 3]>>,
) -> Result<(), Anyhow> {
    while let Ok(n) = games.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
    {
        let game = Game::new(players[(n - 1) % 2].clone(), players[n % 2].clone());
        let pgn = game.play(Position::default()).await?;

        {
            let mut wld = wld.lock().await;
            match pgn.outcome.winner() {
                Some(Color::White) => wld[(n - 1) % 2] += 1,
                Some(Color::Black) => wld[n % 2] += 1,
                _ => wld[2] += 1,
            }

            info!(
                games = wld.iter().sum::<usize>(),
                wins = wld[0],
                losses = wld[1],
                draws = wld[2],
                ΔELO = %ΔELO::new(*wld),
            );
        }

        println!("{pgn}\n");
    }

    Ok(())
}
