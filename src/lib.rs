mod act;
mod action;
mod binary;
mod bits;
mod build;
mod cache;
mod color;
mod eval;
mod fen;
mod file;
mod game;
mod io;
mod limits;
mod r#move;
mod outcome;
mod pgn;
mod piece;
mod position;
mod promotion;
mod rank;
mod register;
mod role;
mod san;
mod search;
mod square;
mod tt;

pub use crate::act::*;
pub use crate::action::*;
pub use crate::binary::*;
pub use crate::bits::*;
pub use crate::build::*;
pub use crate::cache::*;
pub use crate::color::*;
pub use crate::eval::*;
pub use crate::fen::*;
pub use crate::file::*;
pub use crate::game::*;
pub use crate::io::*;
pub use crate::limits::*;
pub use crate::outcome::*;
pub use crate::pgn::*;
pub use crate::piece::*;
pub use crate::position::*;
pub use crate::promotion::*;
pub use crate::r#move::*;
pub use crate::rank::*;
pub use crate::register::*;
pub use crate::role::*;
pub use crate::san::*;
pub use crate::search::*;
pub use crate::square::*;
pub use crate::tt::*;

pub mod engine;
pub mod player;
pub mod strategy;

pub use crate::engine::{Engine, EngineBuilder};
pub use crate::player::{Player, PlayerBuilder, PlayerError};
pub use crate::strategy::{Strategy, StrategyBuilder};

#[cfg(test)]
pub use crate::engine::MockEngineBuilder;

#[cfg(test)]
pub use crate::player::MockPlayerBuilder;

#[cfg(test)]
pub use crate::strategy::MockStrategyBuilder;
