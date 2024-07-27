use crate::{chess::Color, nnue::Feature};

/// Trait for transformer accumulators.
pub trait Accumulator: Default {
    /// The accumulator length.
    const LEN: usize;

    /// Resets this accumulator.
    fn refresh(&mut self, side: Color);

    /// Updates this accumulator by adding a feature.
    fn add(&mut self, side: Color, feature: Feature);

    /// Updates this accumulator by removing a feature.
    fn remove(&mut self, side: Color, feature: Feature);

    /// Updates this accumulator by replacing a feature.
    fn replace(&mut self, side: Color, remove: Feature, add: Feature);

    /// Evaluates this accumulator.
    fn evaluate(&self, turn: Color, phase: usize) -> i32;
}

impl<T: Accumulator, U: Accumulator> Accumulator for (T, U) {
    const LEN: usize = T::LEN + U::LEN;

    #[inline(always)]
    fn refresh(&mut self, side: Color) {
        self.0.refresh(side);
        self.1.refresh(side);
    }

    #[inline(always)]
    fn add(&mut self, side: Color, feature: Feature) {
        self.0.add(side, feature);
        self.1.add(side, feature);
    }

    #[inline(always)]
    fn remove(&mut self, side: Color, feature: Feature) {
        self.0.remove(side, feature);
        self.1.remove(side, feature);
    }

    #[inline(always)]
    fn replace(&mut self, side: Color, remove: Feature, add: Feature) {
        self.0.replace(side, remove, add);
        self.1.replace(side, remove, add);
    }

    #[inline(always)]
    fn evaluate(&self, turn: Color, phase: usize) -> i32 {
        self.0.evaluate(turn, phase) + self.1.evaluate(turn, phase)
    }
}
