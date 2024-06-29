use crate::chess::Color;

/// Trait for transformer accumulators.
pub trait Accumulator {
    /// Refreshes this accumulator.
    fn refresh(&mut self, white: &[u16], black: &[u16]);

    /// Updates this accumulator by adding features.
    fn add(&mut self, white: u16, black: u16);

    /// Updates this accumulator by removing features.
    fn remove(&mut self, white: u16, black: u16);

    /// Evaluates this accumulator.
    fn evaluate(&self, turn: Color, phase: usize) -> i32;
}

impl<T: Accumulator, U: Accumulator> Accumulator for (T, U) {
    fn refresh(&mut self, white: &[u16], black: &[u16]) {
        self.0.refresh(white, black);
        self.1.refresh(white, black);
    }

    fn add(&mut self, white: u16, black: u16) {
        self.0.add(white, black);
        self.1.add(white, black);
    }

    fn remove(&mut self, white: u16, black: u16) {
        self.0.remove(white, black);
        self.1.remove(white, black);
    }

    fn evaluate(&self, turn: Color, phase: usize) -> i32 {
        self.0.evaluate(turn, phase) + self.1.evaluate(turn, phase)
    }
}
