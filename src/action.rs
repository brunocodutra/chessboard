use crate::chess::{Figure, Outcome, Piece, Player, Square};
use derive_more::{Display, Error};

/// A move.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Move {
    pub from: Square,
    pub to: Square,
    /// If the move of a pawn triggers a promotion, the target piece should be specified.
    pub promotion: Option<Piece>,
}

/// The possible actions a player can take.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum PlayerAction {
    /// Move a piece on the board.    
    MakeMove(Player, Move),

    /// Resign the match in favor of the opponent.
    Resign(Player),
}

impl PlayerAction {
    pub fn player(&self) -> &Player {
        use PlayerAction::*;
        match self {
            MakeMove(p, _) | Resign(p) => p,
        }
    }
}

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary), proptest(no_params))]
#[error(ignore)]
pub enum InvalidPlayerAction {
    #[display(fmt = "the game has ended in a {}", "_0")]
    GameHasEnded(Outcome),

    #[display(fmt = "it's not {} player's turn", "_0.color.to_str()")]
    TurnOfTheOpponent(Player),

    #[display(
        fmt = "the {} player is not allowed move the {} {} from {} to {} with {} promotion",
        "_0.color.to_str()",
        "_1.color.to_str()",
        "_1.piece.to_str()",
        "_2.from",
        "_2.to",
        "_2.promotion.map(|p| p.to_str()).unwrap_or(\"no\")"
    )]
    IllegalMove(Player, Figure, Move),

    #[display(
        fmt = "the {} player attempted to move a nonexistent piece from {} to {}",
        "_0.color.to_str()",
        "_1.from",
        "_1.to"
    )]
    InvalidMove(Player, Move),
}
