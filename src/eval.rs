use crate::Position;

/// Trait for types that can evaluate a [`Position`].
#[cfg_attr(test, mockall::automock)]
pub trait Eval {
    /// Evaluates a [`Position`].
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, pos: &Position) -> i16;
}
