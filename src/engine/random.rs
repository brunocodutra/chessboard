use crate::{Eval, Position};
use derive_more::Constructor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A trivial engine that evaluates a [`Position`]s to random, but stable, scores.
#[derive(Debug, Default, Clone, Constructor)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Random {}

impl Eval for Random {
    fn eval(&self, pos: &Position) -> i16 {
        let mut hashser = DefaultHasher::new();
        pos.hash(&mut hashser);
        hashser.finish() as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(pos: Position) {
        assert_eq!(Random::new().eval(&pos), Random::new().eval(&pos.clone()));
    }
}
