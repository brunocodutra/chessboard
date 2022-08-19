use super::Execute;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use chessboard::Search as _;
use chessboard::{Build, Color, Fen, Position, SearchLimits, Strategy, StrategyBuilder};
use clap::{AppSettings::DeriveDisplayOrder, Parser};
use tokio::task::block_in_place;
use tracing::{info, instrument};

/// Searches for the principal variation in a position.
#[derive(Debug, Parser)]
#[clap(
    disable_help_flag = true,
    disable_version_flag = true,
    setting = DeriveDisplayOrder
)]
pub struct Search {
    /// How deep/long to search.
    #[clap(short, long, default_value = "none")]
    limits: SearchLimits,

    /// The search strategy.
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

        match self.limits {
            l @ SearchLimits::Time(_) => block_in_place(|| search(&mut strategy, &pos, l))?,
            l => {
                for d in 0..=l.depth() {
                    block_in_place(|| search(&mut strategy, &pos, SearchLimits::Depth(d)))?
                }
            }
        }

        Ok(())
    }
}

fn search(strategy: &mut Strategy, pos: &Position, limits: SearchLimits) -> Result<(), Anyhow> {
    let pv: Vec<_> = strategy.search(pos, limits).collect();

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

    Ok(())
}
