use derive_more::{DebugCustom, From};
use test_strategy::Arbitrary;

mod pesto;
mod pst;
mod random;

pub use pesto::*;
pub use pst::*;
pub use random::*;

/// Trait for types that can evaluate other types.
pub trait Eval<T> {
    /// Evaluates an item.
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, item: &T) -> i16;
}

/// A generic evaluator.
#[derive(DebugCustom, Clone, Arbitrary, From)]
pub enum Evaluator {
    #[debug(fmt = "{:?}", _0)]
    Random(Random),
    #[debug(fmt = "{:?}", _0)]
    Pesto(Pesto),
}

impl Default for Evaluator {
    fn default() -> Self {
        Evaluator::Pesto(Pesto::default())
    }
}

impl<T> Eval<T> for Evaluator
where
    Random: Eval<T>,
    Pesto: Eval<T>,
{
    fn eval(&self, item: &T) -> i16 {
        match self {
            Evaluator::Random(e) => e.eval(item),
            Evaluator::Pesto(e) => e.eval(item),
        }
    }
}
