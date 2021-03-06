use crate::{Action, Player, Position, Search};
use async_trait::async_trait;
use derive_more::Constructor;
use std::{convert::Infallible, fmt::Debug};
use tracing::instrument;

#[derive(Debug, Constructor)]
pub struct Ai<S: Search + Debug> {
    strategy: S,
}

#[async_trait]
impl<S: Search + Debug + Send> Player for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "trace", err)]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        let mv = self.strategy.search(pos).await.map(Into::into);
        Ok(mv.unwrap_or_else(|| Action::Resign(pos.turn())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{search::MockSearch, Move};
    use proptest::prelude::*;
    use smol::block_on;

    proptest! {
        #[test]
        fn play_searches_for_move(pos: Position, m: Move) {
            let mut strategy = MockSearch::new();
            strategy.expect_search().times(1).returning(move |_| Some(m));

            let mut ai = Ai::new(strategy);
            assert_eq!(block_on(ai.act(&pos)).unwrap(), Action::Move(m));
        }

        #[test]
        fn play_resigns_if_there_are_no_moves(pos: Position) {
            let mut strategy = MockSearch::new();
            strategy.expect_search().times(1).returning(|_| None);

            let mut ai = Ai::new(strategy);
            assert_eq!(block_on(ai.act(&pos)).unwrap(), Action::Resign(pos.turn()));
        }
    }
}
