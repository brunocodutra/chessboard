use crate::{Act, Action, Position, Search};
use async_trait::async_trait;
use derive_more::{Constructor, From};
use std::convert::Infallible;
use tokio::task::block_in_place;
use tracing::instrument;

/// A computed controlled player.
#[derive(Debug, From, Constructor)]
pub struct Ai<S: Search> {
    strategy: S,
}

#[async_trait]
impl<S: Search + Send> Act for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "trace", err, ret, skip(self))]
    async fn act(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        match block_in_place(|| self.strategy.search(pos)) {
            Some(m) => Ok(m.into()),
            None => Ok(Action::Resign),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockSearch, Move};
    use mockall::predicate::eq;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn searches_for_move(pos: Position, m: Move) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy
            .expect_search()
            .once()
            .with(eq(pos.clone()))
            .return_const(Some(m));

        let mut ai = Ai::new(strategy);
        assert_eq!(rt.block_on(ai.act(&pos))?, Action::Move(m));
    }

    #[proptest]
    fn resigns_if_there_are_no_moves(pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy
            .expect_search()
            .once()
            .with(eq(pos.clone()))
            .return_const(None);

        let mut ai = Ai::new(strategy);
        assert_eq!(rt.block_on(ai.act(&pos))?, Action::Resign);
    }
}
