use crate::{Eval, Position};
use derive_more::Constructor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A trivial engine that evaluates [`Position`]s to random, but stable, scores.
#[derive(Debug, Default, Constructor)]
pub struct Random {}

impl Eval for Random {
    fn eval(&self, pos: &Position) -> i32 {
        let mut hashser = DefaultHasher::new();
        pos.hash(&mut hashser);
        hashser.finish() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn evaluate_returns_stable_score(pos: Position) {
        assert_eq!(Random::new().eval(&pos), Random::new().eval(&pos.clone()));
    }
}
