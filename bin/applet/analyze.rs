use crate::{build::Build, engine::EngineConfig, player::Player};
use anyhow::Error as Anyhow;
use clap::Parser;
use futures_util::TryStreamExt;
use lib::chess::{Color, Fen};
use lib::search::Limits;
use tracing::{info, instrument};

/// Analyzes a position.
#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Analyze {
    /// The engine used to analyze the position.
    #[clap(short, long, default_value_t)]
    engine: EngineConfig,

    /// Search limits to use.
    #[clap(short, long, default_value_t)]
    limits: Limits,

    /// The position to analyze in FEN notation.
    fen: Fen,
}

impl Analyze {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let pos = self.fen.try_into()?;
        let mut engine = self.engine.build()?;
        let mut analysis = engine.analyze(&pos, self.limits);

        while let Some(r) = analysis.try_next().await? {
            info!(
                depth = %r.depth(),
                score = %match pos.turn() {
                    Color::White => r.score(),
                    Color::Black => -r.score(),
                },
                pv = %r.pv(),
            );
        }

        Ok(())
    }
}
