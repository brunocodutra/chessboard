use crate::Game;

/// Trait for types that can evaluate a [`Game`].
#[cfg_attr(test, mockall::automock)]
pub trait Eval {
    /// Evaluates a [`Game`].
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, game: &Game) -> i16;
}
