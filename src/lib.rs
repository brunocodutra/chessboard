#![cfg_attr(test, allow(clippy::unit_arg))]

mod action;
mod color;
mod figure;
mod file;
mod foreign;
mod game;
mod outcome;
mod piece;
mod placement;
mod player;
mod promotion;
mod rank;
mod square;

pub use crate::action::*;
pub use crate::color::*;
pub use crate::figure::*;
pub use crate::file::*;
pub use crate::game::*;
pub use crate::outcome::*;
pub use crate::piece::*;
pub use crate::placement::*;
pub use crate::player::*;
pub use crate::promotion::*;
pub use crate::rank::*;
pub use crate::square::*;

pub mod actor;
pub mod remote;

pub use crate::actor::*;
pub use crate::remote::*;
