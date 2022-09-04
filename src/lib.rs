mod build;
mod game;
mod limits;
mod metrics;
mod pv;
mod search;
mod transposition;

pub use build::*;
pub use game::*;
pub use limits::*;
pub use metrics::*;
pub use pv::*;
pub use search::*;
pub use transposition::*;

/// Types that represent the domain model of chess.
pub mod chess;
/// Types that can evaluate chess positions.
pub mod eval;
/// Types that can analyze chess positions.
pub mod play;
pub mod strategy;
/// Assorted utilities.
pub mod util;

pub use eval::Eval;
pub use play::Play;
pub use strategy::{Strategy, StrategyBuilder};

#[cfg(test)]
pub use eval::MockEval;

#[cfg(test)]
pub use play::{MockPlay, MockPlayerBuilder};
