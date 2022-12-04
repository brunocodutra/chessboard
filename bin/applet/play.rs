use crate::{engine::EngineConfig, game::Game};
use anyhow::Error as Anyhow;
use clap::Parser;
use derive_more::{DebugCustom, Display};
use lib::chess::Position;
use std::num::NonZeroUsize;
use tracing::{info, instrument};

/// A match of chess between two players.
#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Play {
    /// How many games to play.
    #[clap(short = 'n', long, default_value = "1")]
    games: NonZeroUsize,

    /// The challenging player starts with the white pieces.
    challenger: EngineConfig,

    /// The defending player starts with the black pieces.
    defender: EngineConfig,
}

#[derive(DebugCustom, Display, Copy, Clone)]
#[debug(fmt = "{}", self)]
#[display(fmt = "[{:+.2}, {:+.2}]", _0, _1)]
struct ΔELO(f64, f64);

impl ΔELO {
    fn new(wins: f64, losses: f64, draws: f64) -> Self {
        let n = wins + losses + draws;
        let w = wins / n;
        let l = losses / n;
        let d = draws / n;

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

impl Play {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let players = [self.challenger, self.defender];
        let (mut wins, mut losses, mut draws) = (0, 0, 0);
        let mut pgns = Vec::with_capacity(self.games.into());

        for n in 0..self.games.into() {
            let game = Game::new(players[n % 2].clone(), players[(n + 1) % 2].clone());
            let pgn = game.play(Position::default()).await?;

            let wl = [&mut wins, &mut losses];
            match pgn.outcome.winner() {
                Some(c) => *wl[(n + c as usize) % 2] += 1,
                _ => draws += 1,
            }

            info!(
                games = wins + losses + draws,
                wins,
                losses,
                draws,
                ΔELO = %ΔELO::new(wins as _, losses as _, draws as _),
            );

            pgns.push(pgn);
        }

        for pgn in pgns {
            println!("{}\n", pgn);
        }

        Ok(())
    }
}
