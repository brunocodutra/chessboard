use super::Eval;
use crate::chess::Position;
use derive_more::Constructor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use test_strategy::Arbitrary;

/// Evaluates a [`Position`]s to random, but stable, scores.
#[derive(Debug, Default, Clone, Arbitrary, Constructor)]
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
