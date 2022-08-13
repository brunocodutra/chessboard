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
        match block_in_place(|| self.strategy.search(pos).next()) {
            Some(t) => Ok(Action::Move(t.best())),
            None => Ok(Action::Resign),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use proptest::prop_assume;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn searches_for_best_move(s: Strategy, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let t = s.search(&pos).next();
        prop_assume!(t.is_some());

        let best = t.unwrap().best();
        let mut ai = Ai::new(s);
        assert_eq!(rt.block_on(ai.act(&pos))?, Action::Move(best));
    }

    #[proptest]
    fn resigns_if_there_are_no_moves(
        s: Strategy,
        #[by_ref]
        #[filter(#pos.moves().len() == 0)]
        pos: Position,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut ai = Ai::new(s);
        assert_eq!(rt.block_on(ai.act(&pos))?, Action::Resign);
    }
}
