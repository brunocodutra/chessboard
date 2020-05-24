#![cfg_attr(test, allow(clippy::unit_arg))]

mod action;
mod color;
mod figure;
mod file;
mod foreign;
mod game;
mod outcome;
mod piece;
mod player;
mod position;
mod rank;
mod square;

pub use crate::action::*;
pub use crate::color::*;
pub use crate::figure::*;
pub use crate::file::*;
pub use crate::game::*;
pub use crate::outcome::*;
pub use crate::piece::*;
pub use crate::player::*;
pub use crate::position::*;
pub use crate::rank::*;
pub use crate::square::*;
