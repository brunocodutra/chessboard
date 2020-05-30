use crate::{color::*, foreign, player::*};
use derive_more::Display;

/// One of the possible outcomes of a chess game.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Outcome {
    #[display(fmt = "resignation by the {} player", "_0.color")]
    Resignation(Player),

    #[display(fmt = "checkmate by the {} player", "_0.color")]
    Checkmate(Player),

    #[display(fmt = "stalemate")]
    Stalemate,

    #[display(fmt = "draw")]
    Draw,
}

impl From<foreign::GameResult> for Outcome {
    fn from(r: foreign::GameResult) -> Self {
        use Color::*;
        use Outcome::*;
        match r {
            foreign::GameResult::WhiteResigns => Resignation(Player { color: White }),
            foreign::GameResult::BlackResigns => Resignation(Player { color: Black }),
            foreign::GameResult::WhiteCheckmates => Checkmate(Player { color: White }),
            foreign::GameResult::BlackCheckmates => Checkmate(Player { color: Black }),
            foreign::GameResult::Stalemate => Stalemate,
            foreign::GameResult::DrawAccepted | foreign::GameResult::DrawDeclared => Draw,
        }
    }
}

impl Into<foreign::GameResult> for Outcome {
    fn into(self) -> foreign::GameResult {
        use Color::*;
        use Outcome::*;
        match self {
            Resignation(Player { color: White }) => foreign::GameResult::WhiteResigns,
            Resignation(Player { color: Black }) => foreign::GameResult::BlackResigns,
            Checkmate(Player { color: White }) => foreign::GameResult::WhiteCheckmates,
            Checkmate(Player { color: Black }) => foreign::GameResult::BlackCheckmates,
            Stalemate => foreign::GameResult::Stalemate,
            Draw => foreign::GameResult::DrawDeclared,
        }
    }
}
