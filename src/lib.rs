#![cfg_attr(test, allow(clippy::unit_arg))]

mod action;
mod color;
mod file;
mod foreign;
mod game;
mod outcome;
mod piece;
mod placement;
mod position;
mod promotion;
mod rank;
mod role;
mod square;

pub use crate::action::*;
pub use crate::color::*;
pub use crate::file::*;
pub use crate::game::*;
pub use crate::outcome::*;
pub use crate::piece::*;
pub use crate::placement::*;
pub use crate::position::*;
pub use crate::promotion::*;
pub use crate::rank::*;
pub use crate::role::*;
pub use crate::square::*;

pub mod player;
pub mod remote;

pub use crate::player::{Player, PlayerDispatcher};
pub use crate::remote::{Remote, RemoteDispatcher, RemoteDispatcherError};
