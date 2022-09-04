mod build;
mod eval;
mod game;
mod limits;
mod metrics;
mod play;
mod pv;
mod search;
mod transposition;

pub use build::*;
pub use eval::*;
pub use game::*;
pub use limits::*;
pub use metrics::*;
pub use play::*;
pub use pv::*;
pub use search::*;
pub use transposition::*;

/// Types that represent the domain model of chess.
pub mod chess;
pub mod engine;
pub mod player;
pub mod strategy;
/// Assorted utilities.
pub mod util;

pub use engine::{Engine, EngineBuilder};
pub use player::{Player, PlayerBuilder, PlayerError};
pub use strategy::{Strategy, StrategyBuilder};

#[cfg(test)]
pub use player::MockPlayerBuilder;
