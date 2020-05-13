use crate::chess::{Figure, Outcome, Piece, Player, Square};
use thiserror::Error;

/// Denotes a move.
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary), proptest(no_params))]
pub enum InvalidPlayerAction {
    #[error("the game has ended in a {}", .0)]
    GameHasEnded(Outcome),

    #[error("it's not {} player's turn", .0.color.to_str())]
    TurnOfTheOpponent(Player),

    #[error("the {} player is not allowed move the {} {} from {} to {} with {} promotion", 
        .0.color.to_str(), .1.color.to_str(), .1.piece.to_str(), .2.from, .2.to,
        .2.promotion.map(|p| p.to_str()).unwrap_or("no"))]
    IllegalMove(Player, Figure, Move),

    #[error("the {} player attempted to move a nonexistent piece from {} to {}", 
        .0.color.to_str(),.1.from, .1.to)]
    InvalidMove(Player, Move),
}
