use super::Execute;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use chessboard::{Build, Color, Fen, Search as _, StrategyBuilder};
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
        let pv: Vec<_> = strategy.search(&pos).collect();

        let head = pv
            .first()
            .copied()
            .with_context(|| format!("search limits may be too low\n{:#?}", strategy.limits()))
            .context("no principal variation found")?;

        let moves: Vec<_> = pv.into_iter().map(|t| t.best().to_string()).collect();

        info!(
            depth = head.draft(),
            score = match pos.turn() {
                Color::White => head.score(),
                Color::Black => -head.score(),
            },
            pv = %moves.join(" ")
        );

        Ok(())
    }
}
