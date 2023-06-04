use crate::{analyze::Analyze as _, engine::Ai};
use anyhow::Error as Anyhow;
use clap::Parser;
use futures_util::TryStreamExt;
use lib::chess::{Color, Fen};
use lib::search::{Limits, Options};
use tracing::{info, instrument};

/// Analyzes a position.
#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Analyze {
    /// The engine configuration.
    #[clap(short, long, default_value_t)]
    options: Options,

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
        let mut ai = Ai::new(self.options);
        let mut analysis = ai.analyze(&pos, self.limits);

        while let Some(pv) = analysis.try_next().await? {
            info!(
                depth = %pv.depth(),
                score = %match pos.turn() {
                    Color::White => pv.score(),
                    Color::Black => -pv.score(),
                },
                pv = %pv.line(),
            );
        }

        Ok(())
    }
}
