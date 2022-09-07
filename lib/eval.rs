use crate::chess::Position;
use derive_more::{DebugCustom, From};
use test_strategy::Arbitrary;

mod materialist;
mod pesto;
mod pst;
mod random;

pub use materialist::*;
pub use pesto::*;
pub use pst::*;
pub use random::*;

/// Trait for types that can evaluate a [`Position`].
pub trait Eval {
    /// Evaluates a [`Position`].
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, pos: &Position) -> i16;
}

/// A generic [`Position`] evaluator.
#[derive(DebugCustom, Clone, Arbitrary, From)]
pub enum Evaluator {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[debug(fmt = "{:?}", _0)]
    Materialist(Materialist),
    #[debug(fmt = "{:?}", _0)]
    Pesto(Pesto),
}

impl Default for Evaluator {
    fn default() -> Self {
        Evaluator::Pesto(Pesto::default())
    }
}

impl Eval for Evaluator {
    fn eval(&self, pos: &Position) -> i16 {
        match self {
            Evaluator::Random(e) => e.eval(pos),
            Evaluator::Materialist(e) => e.eval(pos),
            Evaluator::Pesto(e) => e.eval(pos),
        }
    }
}
