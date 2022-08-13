use crate::Move;
use derive_more::{DebugCustom, Display, From};

/// The possible actions a player can take.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Hash, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum Action {
    /// Move a piece on the board.
    #[debug(fmt = "{:?}", _0)]
    #[display(fmt = "{}", _0)]
    Move(Move),

    /// Resign the game in favor of the opponent.
    #[display(fmt = "resign")]
    #[from(ignore)]
    Resign,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn action_can_be_converted_from_move(m: Move) {
        assert_eq!(Action::from(m), Action::Move(m));
    }
}
