use super::Execute;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use chessboard::chess::{Color, Fen};
use chessboard::engine::Builder as EngineBuilder;
use chessboard::prelude::*;
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use futures_util::TryStreamExt;
use tracing::{info, instrument};

/// Analyzes a position.
#[derive(Debug, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Analyze {
    /// The engine used to analyze.
    engine: EngineBuilder,

    /// The position to search in FEN notation.
    fen: Fen,
}

#[async_trait]
impl Execute for Analyze {
    #[instrument(level = "trace", skip(self), err)]
    async fn execute(self) -> Result<(), Anyhow> {
        let pos = self.fen.try_into()?;
        let mut engine = self.engine.build()?;
        let mut analysis = engine.analyze(&pos);

        while let Some(pv) = analysis.try_next().await? {
            let (d, s) =
                Option::zip(pv.depth(), pv.score()).context("no principal variation found")?;

            info!(
                depth = d,
                score = match pos.turn() {
                    Color::White => s,
                    Color::Black => -s,
                },
                %pv
            );
        }

        Ok(())
    }
}
