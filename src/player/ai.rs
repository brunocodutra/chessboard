use crate::{Act, Action, Game, Search, SearchControl};
use async_trait::async_trait;
use derive_more::{Constructor, From};
use std::{convert::Infallible, fmt::Debug};
use tokio::task::block_in_place;
use tracing::instrument;

/// A computed controlled player.
#[derive(Debug, From, Constructor)]
pub struct Ai<S: Search> {
    strategy: S,
}

#[async_trait]
impl<S: Search + Debug + Send> Act for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "trace", err, ret)]
    async fn act(&mut self, game: &Game) -> Result<Action, Self::Error> {
        let ctrl = SearchControl::default();
        Ok(block_in_place(|| self.strategy.search(game, ctrl)).unwrap_or(Action::Resign))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockSearch;
    use mockall::predicate::{always, eq};
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn searches_for_move(g: Game, a: Action) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy
            .expect_search()
            .once()
            .with(eq(g.clone()), always())
            .return_const(Some(a));

        let mut ai = Ai::new(strategy);
        assert_eq!(rt.block_on(ai.act(&g))?, a);
    }

    #[proptest]
    fn resigns_if_there_are_no_moves(g: Game) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy
            .expect_search()
            .once()
            .with(eq(g.clone()), always())
            .return_const(None);

        let mut ai = Ai::new(strategy);
        assert_eq!(rt.block_on(ai.act(&g))?, Action::Resign);
    }
}
