use crate::Color;
use derive_more::Display;

/// One of the possible outcomes of a chess game.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum Outcome {
    #[display(fmt = "resignation by the {} player", _0)]
    Resignation(Color),

    #[display(fmt = "checkmate by the {} player", _0)]
    Checkmate(Color),

    #[display(fmt = "stalemate")]
    Stalemate,

    #[display(fmt = "draw")]
    Draw,
}
