use super::Execute;
use anyhow::Error as Anyhow;
use async_trait::async_trait;
use chessboard::{Build, Color, Game, PlayerBuilder};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use libm::erf;
use std::num::NonZeroUsize;
use tracing::info;

/// A match of chess between two players.
#[derive(Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Play {
    /// White pieces player.
    #[clap(short, long, value_name = "player", default_value = "cli()")]
    white: PlayerBuilder,

    /// Black pieces player.
    #[clap(short, long, value_name = "player", default_value = "cli()")]
    black: PlayerBuilder,

    /// How many games to play.
    #[clap(short = 'n', long, value_name = "number", default_value = "1")]
    games: NonZeroUsize,
}

#[async_trait]
impl Execute for Play {
    async fn execute(&self) -> Result<(), Anyhow> {
        let (mut wins, mut losses, mut draws) = (0f64, 0f64, 0f64);
        let mut reports = Vec::with_capacity(self.games.into());

        for n in 0..self.games.into() {
            let white = self.white.clone().build()?;
            let black = self.black.clone().build()?;
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
}
