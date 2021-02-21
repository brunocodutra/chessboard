use crate::{Color, Move, Outcome, Position};
use derive_more::{Display, Error, From};

/// The possible actions a player can take.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Action {
    /// Move a piece on the board.
    #[display(fmt = "move {}", _0)]
    Move(Move),

    /// Resign the game in favor of the opponent.
    #[display(fmt = "resign")]
    Resign,
}

/// Represents an illegal [`Move`] in a given [`Position`].
#[derive(Debug, Display, Clone, /*Eq,*/ PartialEq, Hash, Error)]
#[display(fmt = "position `{}` does not permit move `{}`", _1, _0)]
pub struct IllegalMove(pub Move, pub Position);

/// The reason why the player [`Action`] was rejected.
#[derive(Debug, Display, Clone, /*Eq,*/ PartialEq, Hash, Error)]
#[error(ignore)]
pub enum InvalidAction {
    #[display(fmt = "the game has ended in a {}", _0)]
    GameHasEnded(Outcome),

    #[display(fmt = "the {} player attempted an illegal move", _0)]
    PlayerAttemptedIllegalMove(Color, #[error(source)] IllegalMove),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn move_can_be_converted_into_action(m: Move) {
            assert_eq!(Action::from(m), Action::Move(m));
        }
    }
}
