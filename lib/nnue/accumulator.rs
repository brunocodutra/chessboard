/// Trait for transformer accumulators.
pub trait Accumulator {
    /// Mirrors the accumulator.
    fn mirror(&mut self);

    /// Refreshes accumulator.
    fn refresh(&mut self, us: &[u16], them: &[u16]);

    /// Updates the accumulator by adding features.
    fn add(&mut self, us: u16, them: u16);

    /// Updates the accumulator by removing features.
    fn remove(&mut self, us: u16, them: u16);

    /// Evaluates the accumulator.
    fn evaluate(&self, phase: usize) -> i32;
}

impl<T: Accumulator, U: Accumulator> Accumulator for (T, U) {
    fn mirror(&mut self) {
        self.0.mirror();
        self.1.mirror();
    }

    fn refresh(&mut self, us: &[u16], them: &[u16]) {
        self.0.refresh(us, them);
        self.1.refresh(us, them);
    }

    fn add(&mut self, us: u16, them: u16) {
        self.0.add(us, them);
        self.1.add(us, them);
    }

    fn remove(&mut self, us: u16, them: u16) {
        self.0.remove(us, them);
        self.1.remove(us, them);
    }

    fn evaluate(&self, phase: usize) -> i32 {
        self.0.evaluate(phase) + self.1.evaluate(phase)
    }
}
