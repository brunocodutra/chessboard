#![feature(const_maybe_uninit_write, const_mut_refs, const_transmute_copy)]

use chess::{Position, Square};
use num_traits::PrimInt;
use std::ops::Range;
use util::Value;

mod affine;
mod chain;
mod crelu;
mod damp;
mod feature;
mod nn;
mod psqt;
mod transformer;

use affine::*;
use chain::*;
use crelu::*;
use damp::*;
use feature::*;
use nn::*;
use psqt::*;
use transformer::*;

/// Trait for types that can compose a neural network.
trait Layer<Input> {
    /// The transformed neurons.
    type Output;

    /// Transforms input neurons.
    fn forward(&self, input: Input) -> Self::Output;
}

/// Trait for types that can incrementally transform features.
trait Transformer {
    /// A type that can accumulate transformed features.
    type Accumulator;

    /// Refreshes accumulator.
    fn refresh(&self, features: &[usize], accumulator: &mut Self::Accumulator);

    /// Updates the accumulator by adding features.
    fn add(&self, feature: usize, accumulator: &mut Self::Accumulator);

    /// Updates the accumulator by removing features.
    fn remove(&self, feature: usize, accumulator: &mut Self::Accumulator);
}

impl<T: Transformer<Accumulator = [U; N]>, U: PrimInt, const N: usize> Layer<&[usize]> for T {
    type Output = [U; N];

    #[inline]
    fn forward(&self, input: &[usize]) -> Self::Output {
        let mut accumulator = [U::zero(); N];
        self.refresh(input, &mut accumulator);
        accumulator
    }
}

/// Evaluates a [`Position`] using a trained NNUE.
///
/// Positive values favor the current side to play.
pub fn eval(pos: &Position) -> Value {
    let us = Nnue::perspective(pos, pos.turn());
    let them = Nnue::perspective(pos, !pos.turn());
    let phase = (pos.occupied().len() - 1) / 4;
    let material = NNUE.material(phase, &us, &them);
    let positional = NNUE.positional(phase, &us, &them);
    Value::saturate(material + positional)
}

/// The Static Exchange Evaluation ([SEE]) algorithm.
///
/// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
pub fn see(mut pos: Position, square: Square, bounds: Range<Value>) -> Value {
    assert!(!bounds.is_empty(), "{bounds:?} ≠ ∅");

    let (alpha, beta) = (bounds.start, bounds.end);
    let alpha = eval(&pos).max(alpha);

    if alpha >= beta {
        return beta;
    }

    match pos.exchange(square) {
        Ok(_) => -see(pos, square, -beta..-alpha),
        Err(_) => alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn see_returns_value_within_bounds(
        pos: Position,
        s: Square,
        #[filter(!#r.is_empty())] r: Range<Value>,
    ) {
        let (a, b) = (r.start, r.end);
        assert!((a..=b).contains(&see(pos, s, r)));
    }
}
