use crate::{Eval, Game};
use derive_more::Constructor;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A trivial engine that evaluates a [`Game`]s to random, but stable, scores.
#[derive(Debug, Default, Constructor)]
pub struct Random {}

impl Eval for Random {
    fn eval(&self, game: &Game) -> i32 {
        let mut hashser = DefaultHasher::new();
        game.hash(&mut hashser);
        hashser.finish() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(game: Game) {
        assert_eq!(Random::new().eval(&game), Random::new().eval(&game.clone()));
    }
}
