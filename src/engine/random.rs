use crate::{Engine, Position};
use tracing::instrument;

pub use crate::random::Random;

impl Engine for Random {
    #[instrument(level = "trace")]
    fn evaluate(&self, _: &Position) -> i32 {
        self.gen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rand::rngs::mock::StepRng;

    proptest! {
        #[test]
        fn evaluate_returns_random_number(pos: Position, val: i32) {
            let engine = Random::new(StepRng::new(val as u64, 0));
            assert_eq!(engine.evaluate(&pos), val);
        }
    }
}
