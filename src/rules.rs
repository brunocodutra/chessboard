use crate::action::{InvalidPlayerAction, PlayerAction};

pub mod standard;

pub trait ChessRules {
    fn execute(&mut self, action: PlayerAction) -> Result<(), InvalidPlayerAction>;
}
