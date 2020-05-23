use crate::{action::*, chess::*};

mod standard;

pub use standard::*;

/// Trait for types that implement the rules of a variant of chess.
pub trait ChessRules {
    /// Executes `action` if valid, otherwise returns the reason why not.
    fn execute(&mut self, action: PlayerAction) -> Result<(), InvalidPlayerAction>;

    /// `Some(Outcome)` is the game has ended or `None`.
    fn outcome(&self) -> Option<Outcome>;

    /// The current position.
    fn position(&self) -> Position;
}
