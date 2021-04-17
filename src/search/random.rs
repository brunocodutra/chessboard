use crate::{Move, Position, Search};
use async_trait::async_trait;
use rand::{rngs::StdRng, seq::IteratorRandom, SeedableRng};

#[derive(Debug)]
pub struct Random {
    rng: StdRng,
}

#[async_trait]
impl Search for Random {
    async fn search(&mut self, pos: &Position) -> Option<Move> {
        pos.moves().into_iter().choose(&mut self.rng)
    }
}

impl Default for Random {
    fn default() -> Self {
        Random {
            rng: StdRng::from_entropy(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use smol::block_on;
    use std::collections::HashSet;

    proptest! {
        #[test]
        fn search_returns_a_valid_move(pos: Position) {
            let mvs: HashSet<_> = pos.moves().into_iter().collect();

            if let Some(mv) = block_on(Random::default().search(&pos)) {
                assert_eq!(Some(&mv), mvs.get(&mv));
            } else {
                assert_eq!(HashSet::default(), mvs);
            }
        }
    }
}
