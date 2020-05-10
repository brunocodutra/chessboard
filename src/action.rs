use crate::chess::{Piece, Square};

/// Denotes a move.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    /// If the move of a pawn triggers a promotion, the target piece should be specified.
    pub promotion: Option<Piece>,
}

/// The possible actions a player can take.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PlayerAction {
    /// Move a piece on the board.    
    MakeMove(Player, Move),

    /// Resign the match in favor of the opponent.
    Resign(Player),
}
