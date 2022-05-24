use crate::{Action, Play, Position, Search};
use async_trait::async_trait;
use derive_more::{Constructor, From};
use std::{convert::Infallible, fmt::Debug};
use tracing::instrument;

/// A computed controlled player.
#[derive(Debug, From, Constructor)]
pub struct Ai<S: Search> {
    strategy: S,
}

#[async_trait]
impl<S: Search + Debug + Send> Play for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "trace", err)]
    async fn play(&mut self, pos: &Position) -> Result<Action, Self::Error> {
        let mv = self.strategy.search(pos).map(Into::into);
        Ok(mv.unwrap_or(Action::Resign))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockSearch, Move};
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn play_searches_for_move(pos: Position, m: Move) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy.expect_search().once().returning(move |_| Some(m));

        let mut ai = Ai::new(strategy);
        assert_eq!(rt.block_on(ai.play(&pos))?, Action::Move(m));
    }

    #[proptest]
    fn play_resigns_if_there_are_no_moves(pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy.expect_search().once().returning(|_| None);

        let mut ai = Ai::new(strategy);
        assert_eq!(rt.block_on(ai.play(&pos))?, Action::Resign);
    }
}
