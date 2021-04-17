use crate::{Move, Position, Search};
use async_trait::async_trait;
use tracing::instrument;

pub use crate::random::Random;

#[async_trait]
impl Search for Random {
    #[instrument(level = "trace")]
    async fn search(&mut self, pos: &Position) -> Option<Move> {
        pos.moves().into_iter().nth(self.gen::<u8>().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rand::rngs::mock::StepRng;
    use smol::block_on;

    proptest! {
        #[test]
        fn search_randomly_samples_valid_move(pos: Position, n: u8) {
            let mv = block_on(Random::new(StepRng::new(n.into(), 0)).search(&pos));
            assert_eq!(mv, pos.moves().into_iter().nth(n.into()));
        }
    }
}
