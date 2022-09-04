mod build;
mod game;
mod metrics;
mod pv;
mod transposition;

pub use build::*;
pub use game::*;
pub use metrics::*;
pub use pv::*;
pub use transposition::*;

/// Types that represent the domain model of chess.
pub mod chess;
/// Types that can evaluate chess positions.
pub mod eval;
/// Types that can analyze chess positions.
pub mod play;
/// Types related to adversarial search.
pub mod search;
/// Assorted utilities.
pub mod util;

pub use eval::Eval;
pub use play::Play;
pub use search::Search;

#[cfg(test)]
pub use eval::MockEval;

#[cfg(test)]
pub use play::{MockPlay, MockPlayerBuilder};

#[cfg(test)]
pub use search::MockSearch;
