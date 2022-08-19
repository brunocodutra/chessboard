use std::time::Duration;

use super::Execute;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use chessboard::{Build, Color, Fen, Search as _, SearchLimits, StrategyBuilder};
use clap::{AppSettings::DeriveDisplayOrder, Parser};

use tracing::{info, instrument};

/// Searches for the principal variation in a position.
#[derive(Debug, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Search {
    #[clap(short, long, value_name = "depth", default_value = "255")]
    depth: u8,

    /// The search algorithm to use.
    strategy: StrategyBuilder,

    /// The position to search in FEN notation.
    fen: Fen,
}

#[async_trait]
impl Execute for Search {
    #[instrument(level = "trace", skip(self), err)]
    async fn execute(self) -> Result<(), Anyhow> {
        let mut strategy = self.strategy.build()?;
        let pos = self.fen.try_into()?;

        for depth in 0..=self.depth {
            let limits = SearchLimits {
                depth,
                time: Duration::MAX,
            };

            let pv: Vec<_> = strategy.search(&pos, limits).collect();

            let head = *pv.first().context("no principal variation found")?;
            let moves: Vec<_> = pv.into_iter().map(|t| t.best().to_string()).collect();

            info!(
                depth = head.draft(),
                score = match pos.turn() {
                    Color::White => head.score(),
                    Color::Black => -head.score(),
                },
                pv = %moves.join(" ")
            );
        }

        Ok(())
    }
}
