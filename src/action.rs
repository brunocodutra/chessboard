use crate::{Color, IllegalMove, Move, Outcome};
use derive_more::{Display, Error, From};

/// The possible actions a player can take.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Action {
    /// Move a piece on the board.
    Move(Move),

    /// Resign the game in favor of the opponent.
    #[from(ignore)]
    Resign(Color),
}

/// The reason why the player [`Action`] was rejected.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[error(ignore)]
pub enum InvalidAction {
    #[display(fmt = "the game has already ended in a {}", _0)]
    GameHasEnded(Outcome),

    #[display(fmt = "{}", _0)]
    PlayerAttemptedIllegalMove(IllegalMove),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn action_can_be_converted_from_move(m: Move) {
            assert_eq!(Action::from(m), Action::Move(m));
        }

        #[test]
        fn invalid_action_can_be_converted_from_outcome(o: Outcome) {
            assert_eq!(InvalidAction::from(o), InvalidAction::GameHasEnded(o));
        }

        #[test]
        fn invalid_action_can_be_converted_from_illegal_move(im: IllegalMove) {
            assert_eq!(InvalidAction::from(im.clone()), InvalidAction::PlayerAttemptedIllegalMove(im));
        }
    }
}
