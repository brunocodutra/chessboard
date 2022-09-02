use super::Eval;
use derive_more::Constructor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use test_strategy::Arbitrary;

/// Evaluates a [`Position`]s to random, but stable, scores.
#[derive(Debug, Default, Clone, Arbitrary, Constructor)]
pub struct Random {}

impl<T: Hash> Eval<T> for Random {
    fn eval(&self, item: &T) -> i16 {
        let mut hashser = DefaultHasher::new();
        item.hash(&mut hashser);
        hashser.finish() as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(item: String) {
        assert_eq!(Random::new().eval(&item), Random::new().eval(&item));
    }
}
