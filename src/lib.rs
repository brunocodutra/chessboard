mod build;
mod color;
mod eval;
mod fen;
mod file;
mod game;
mod io;
mod limits;
mod metrics;
mod r#move;
mod outcome;
mod pgn;
mod piece;
mod play;
mod position;
mod promotion;
mod pv;
mod rank;
mod role;
mod san;
mod search;
mod square;
mod transposition;

pub use crate::build::*;
pub use crate::color::*;
pub use crate::eval::*;
pub use crate::fen::*;
pub use crate::file::*;
pub use crate::game::*;
pub use crate::io::*;
pub use crate::limits::*;
pub use crate::metrics::*;
pub use crate::outcome::*;
pub use crate::pgn::*;
pub use crate::piece::*;
pub use crate::play::*;
pub use crate::position::*;
pub use crate::promotion::*;
pub use crate::pv::*;
pub use crate::r#move::*;
pub use crate::rank::*;
pub use crate::role::*;
pub use crate::san::*;
pub use crate::search::*;
pub use crate::square::*;
pub use crate::transposition::*;

pub mod engine;
pub mod player;
pub mod strategy;
/// Assorted utilities.
pub mod util;

pub use crate::engine::{Engine, EngineBuilder};
pub use crate::player::{Player, PlayerBuilder, PlayerError};
pub use crate::strategy::{Strategy, StrategyBuilder};

#[cfg(test)]
pub use crate::player::MockPlayerBuilder;
