use crate::{Move, Play, Position, Search, SearchLimits};
use async_trait::async_trait;
use derive_more::From;
use std::convert::Infallible;
use tokio::task::block_in_place;
use tracing::instrument;

/// A computed controlled player.
#[derive(Debug, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Ai<S: Search> {
    strategy: S,
    limits: SearchLimits,
}

impl<S: Search> Ai<S> {
    /// Constructs [`Ai`] with default [`SearchLimits`].
    pub fn new(strategy: S) -> Self {
        Ai::with_config(strategy, SearchLimits::default())
    }

    /// Constructs [`Ai`] with some [`SearchLimits`].
    pub fn with_config(strategy: S, limits: SearchLimits) -> Self {
        Ai { strategy, limits }
    }
}

#[async_trait]
impl<S: Search + Send> Play for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos))]
    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error> {
        let best = block_in_place(|| Some(self.strategy.search(pos, self.limits).next()?.best()));
        Ok(best.expect("expected non-terminal position"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockSearch, Strategy};
    use proptest::prop_assume;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn new_applies_default_search_limits() {
        assert_eq!(Ai::new(MockSearch::new()).limits, SearchLimits::default());
    }

    #[proptest]
    fn searches_for_best_move(mut ai: Ai<Strategy>, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let t = ai.strategy.search(&pos, ai.limits).next();
        prop_assume!(t.is_some());

        let best = t.unwrap().best();
        assert_eq!(rt.block_on(ai.play(&pos))?, best);
    }

    #[proptest]
    #[should_panic]
    fn panics_if_there_are_no_moves(
        mut ai: Ai<Strategy>,
        #[by_ref]
        #[filter(#pos.moves().len() == 0)]
        pos: Position,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        rt.block_on(ai.play(&pos))?;
    }
}
