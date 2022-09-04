use crate::chess::{Move, Position};
use crate::{Play, Search, SearchLimits};
use async_trait::async_trait;
use derive_more::From;
use std::convert::Infallible;
use tokio::task::block_in_place;
use tracing::{instrument, Span};

/// A computed controlled player.
#[derive(Debug, Default, From)]
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

    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos, depth, score))]
    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error> {
        let pv = block_in_place(|| self.strategy.search::<1>(pos, self.limits));

        if let Some((d, s)) = Option::zip(pv.depth(), pv.score()) {
            Span::current().record("depth", d).record("score", s);
        }

        Ok(*pv.first().expect("expected at least one legal move"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockSearch, Pv, Transposition};
    use std::iter::once;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn new_applies_default_search_limits() {
        assert_eq!(Ai::new(MockSearch::new()).limits, SearchLimits::default());
    }

    #[proptest]
    fn searches_for_best_move(
        l: SearchLimits,
        pos: Position,
        #[filter(#t.draft() > 0)] t: Transposition,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let pv: Pv<256> = once(t).collect();

        let mut strategy = MockSearch::new();
        strategy.expect_search().return_const(pv);

        let mut ai = Ai::with_config(strategy, l);
        assert_eq!(rt.block_on(ai.play(&pos))?, t.best());
    }

    #[proptest]
    #[should_panic]
    fn panics_if_there_are_no_moves(l: SearchLimits, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy.expect_search().return_const(Pv::default());

        let mut ai = Ai::with_config(strategy, l);
        rt.block_on(ai.play(&pos))?;
    }
}
