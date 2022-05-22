use crate::{Engine, Position};
use derive_more::Constructor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A trivial [`Engine`] that evaluates positions to random, but stable, scores.
#[derive(Debug, Default, Constructor)]
pub struct Random {}

impl Engine for Random {
    fn evaluate(&self, pos: &Position) -> i32 {
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
        assert_eq!(
            Random::new().evaluate(&pos),
            Random::new().evaluate(&pos.clone())
        );
    }
}
