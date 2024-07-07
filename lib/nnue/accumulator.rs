use crate::{chess::Color, nnue::Feature};

/// Trait for transformer accumulators.
pub trait Accumulator: Default {
    /// The accumulator length.
    const LEN: usize;

    /// Updates this accumulator by adding features.
    fn add(&mut self, white: Feature, black: Feature);

    /// Updates this accumulator by removing features.
    fn remove(&mut self, white: Feature, black: Feature);

    /// Updates this accumulator by replacing features.
    fn replace(&mut self, white: [Feature; 2], black: [Feature; 2]);

    /// Evaluates this accumulator.
    fn evaluate(&self, turn: Color, phase: usize) -> i32;
}

impl<T: Accumulator, U: Accumulator> Accumulator for (T, U) {
    const LEN: usize = T::LEN + U::LEN;

    fn add(&mut self, white: Feature, black: Feature) {
        self.0.add(white, black);
        self.1.add(white, black);
    }

    fn remove(&mut self, white: Feature, black: Feature) {
        self.0.remove(white, black);
        self.1.remove(white, black);
    }

    fn replace(&mut self, white: [Feature; 2], black: [Feature; 2]) {
        self.0.replace(white, black);
        self.1.replace(white, black);
    }

    fn evaluate(&self, turn: Color, phase: usize) -> i32 {
        self.0.evaluate(turn, phase) + self.1.evaluate(turn, phase)
    }
}
