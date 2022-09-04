mod game;

pub use game::*;

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

/// Convenience module that brings common traits into scope.
pub mod prelude {
    pub use crate::eval::Eval as _;
    pub use crate::play::Play as _;
    pub use crate::search::Search as _;
    pub use crate::util::Build as _;
}
