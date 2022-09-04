use super::Execute;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use chessboard::chess::{Color, Fen, Position};
use chessboard::prelude::*;
use chessboard::search::{Builder as StrategyBuilder, Dispatcher as Strategy, Limits};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use tokio::task::block_in_place;
use tracing::{info, instrument};

/// Analyzes a position.
#[derive(Debug, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Analyze {
    /// How deep/long to search.
    #[clap(short, long, default_value_t)]
    limits: Limits,

    /// The search strategy.
    #[clap(short, long, default_value_t)]
    strategy: StrategyBuilder,

    /// The position to search in FEN notation.
    fen: Fen,
}

#[async_trait]
impl Execute for Analyze {
    #[instrument(level = "trace", skip(self), err)]
    async fn execute(self) -> Result<(), Anyhow> {
        let mut strategy = self.strategy.build()?;
        let pos = self.fen.try_into()?;

        match self.limits {
            l @ Limits::Time(_) => block_in_place(|| search(&mut strategy, &pos, l))?,
            l => {
                for d in 1..=l.depth() {
                    block_in_place(|| search(&mut strategy, &pos, Limits::Depth(d)))?
                }
            }
        }

        Ok(())
    }
}

fn search(strategy: &mut Strategy, pos: &Position, limits: Limits) -> Result<(), Anyhow> {
    let pv = strategy.search::<{ u8::MAX as usize }>(pos, limits);
    let (d, s) = Option::zip(pv.depth(), pv.score()).context("no principal variation found")?;

    info!(
        depth = d,
        score = match pos.turn() {
            Color::White => s,
            Color::Black => -s,
        },
        %pv
    );

    Ok(())
}
