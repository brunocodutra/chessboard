use crate::action::{InvalidPlayerAction, PlayerAction};

mod standard;

pub use standard::*;

pub trait ChessRules {
    fn execute(&mut self, action: PlayerAction) -> Result<(), InvalidPlayerAction>;
}
