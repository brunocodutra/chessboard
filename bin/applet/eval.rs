use anyhow::Error as Anyhow;
use clap::Parser;
use lib::chess::{Color, Position};
use lib::nnue::Evaluator;
use tracing::{info, instrument};

/// Statically evaluates a position.
#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_version_flag = true)]
pub struct Eval {
    /// The position to evaluate in FEN notation.
    pos: Position,
}

impl Eval {
    #[instrument(level = "trace", skip(self), err)]
    pub async fn execute(self) -> Result<(), Anyhow> {
        let pos = Evaluator::own(self.pos);

        let (material, positional, value) = match pos.turn() {
            Color::White => (pos.material(), pos.positional(), pos.value()),
            Color::Black => (-pos.material(), -pos.positional(), -pos.value()),
        };

        info!(%material, %positional, %value);

        Ok(())
    }
}
