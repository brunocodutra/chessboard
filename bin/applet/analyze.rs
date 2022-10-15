use crate::{build::Build, engine::EngineConfig, player::Player};
use anyhow::{Context, Error as Anyhow};
use clap::Parser;
use futures_util::TryStreamExt;
use lib::chess::{Color, Fen};
use tracing::{info, instrument};

/// Analyzes a position.
#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Analyze {
    /// The engine used to analyze the position.
    #[clap(short, long, default_value_t)]
    engine: EngineConfig,

    /// The position to analyze in FEN notation.
    fen: Fen,
}

impl Analyze {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let pos = self.fen.try_into()?;
        let mut engine = self.engine.build()?;
        let mut analysis = engine.analyze::<256>(&pos);

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
