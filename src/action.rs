use crate::{Color, Move, Outcome, Piece, Role};
use derive_more::{Display, Error, From};

/// The possible actions a player can take.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum PlayerAction {
    /// Move a piece on the board.
    #[display(fmt = "move {}", _0)]
    MakeMove(Move),

    /// Resign the match in favor of the opponent.
    #[display(fmt = "resign")]
    Resign,
}

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[error(ignore)]
pub enum InvalidPlayerAction {
    #[display(fmt = "the game has ended in a {}", _0)]
    GameHasEnded(Outcome),

    #[display(
        fmt = "the {} player is not allowed to move a {} {} from {} to {} with {} promotion",
        "_0",
        "_1.color()",
        "_1.role()",
        "_2.from",
        "_2.to",
        "Option::<Role>::from(_2.promotion).map_or_else(|| \"no\", |r| r.into())"
    )]
    IllegalMove(Color, Piece, Move),

    #[display(
        fmt = "the {} player attempted to move a nonexistent piece from {} to {}",
        "_0",
        "_1.from",
        "_1.to"
    )]
    InvalidMove(Color, Move),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn move_can_be_converted_into_action(m: Move) {
            assert_eq!(PlayerAction::from(m), PlayerAction::MakeMove(m));
        }
    }
}
