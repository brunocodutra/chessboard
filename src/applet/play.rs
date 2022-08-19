use super::Execute;
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use chessboard::{Game, PlayerBuilder, Position};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use libm::erf;
use std::num::NonZeroUsize;
use tracing::{info, instrument};

/// A match of chess between two players.
#[derive(Debug, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Play {
    /// How many games to play.
    #[clap(short = 'n', long, default_value = "1")]
    games: NonZeroUsize,

    /// The challenging player starts with the white pieces.
    challenger: PlayerBuilder,

    /// The defending player starts with the black pieces.
    defender: PlayerBuilder,
}

#[async_trait]
impl Execute for Play {
    #[instrument(level = "trace", skip(self), err)]
    async fn execute(self) -> Result<(), Anyhow> {
        let players = [self.challenger, self.defender];
        let (mut wins, mut losses, mut draws) = (0f64, 0f64, 0f64);
        let mut pgns = Vec::with_capacity(self.games.into());

        for n in 0..self.games.into() {
            let game = Game::new(players[n % 2].clone(), players[(n + 1) % 2].clone());
            let pgn = game.play(Position::default()).await?;

            let wl = [&mut wins, &mut losses];
            match pgn.outcome.winner() {
                Some(c) => *wl[(n + c as usize) % 2] += 1.,
                _ => draws += 1.,
            }

            info!(
                games = n + 1,
                challenger = wins + draws / 2.,
                defender = losses + draws / 2.,
                ΔELO = -400. * ((wins + losses + draws) / (wins + draws / 2.) - 1.).log10(),
                LOS = (1. + erf((wins - losses) / (2. * (wins + losses)).sqrt())) / 2.
            );

            pgns.push(pgn);
        }

        for pgn in pgns {
            println!("{}\n", pgn);
        }

        Ok(())
    }
}
