use crate::Color;
use derive_more::Display;

#[cfg(test)]
use test_strategy::Arbitrary;

/// One of the possible outcomes of a chess game.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(Arbitrary))]
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
