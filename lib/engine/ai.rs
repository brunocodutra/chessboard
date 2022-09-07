use super::Engine;
use crate::chess::{Move, Position};
use crate::search::{Limits, Pv, Search};
use async_stream::try_stream;
use async_trait::async_trait;
use derive_more::From;
use futures_util::stream::BoxStream;
use std::convert::Infallible;
use std::time::Instant;
use test_strategy::Arbitrary;
use tokio::task::block_in_place;
use tracing::{instrument, Span};

/// A chess engine.
#[derive(Debug, Default, Arbitrary, From)]
pub struct Ai<S: Search> {
    strategy: S,
    limits: Limits,
}

impl<S: Search> Ai<S> {
    /// Constructs [`Ai`] with default [`Limits`].
    pub fn new(strategy: S) -> Self {
        Ai::with_config(strategy, Limits::default())
    }

    /// Constructs [`Ai`] with some [`Limits`].
    pub fn with_config(strategy: S, limits: Limits) -> Self {
        Ai { strategy, limits }
    }
}

#[async_trait]
impl<S: Search + Send> Engine for Ai<S> {
    type Error = Infallible;

    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos, depth, score))]
    async fn play(&mut self, pos: &Position) -> Result<Move, Self::Error> {
        let pv = block_in_place(|| self.strategy.search::<1>(pos, self.limits));

        if let Some((d, s)) = Option::zip(pv.depth(), pv.score()) {
            Span::current().record("depth", d).record("score", s);
        }

        Ok(*pv.first().expect("expected at least one legal move"))
    }

    #[instrument(level = "debug", skip(self, pos), fields(%pos))]
    fn analyze(&mut self, pos: &Position) -> BoxStream<'_, Result<Pv, Self::Error>> {
        let pos = pos.clone();

        Box::pin(try_stream! {
            let timer = Instant::now();
            for d in 1..=self.limits.depth() {
                let elapsed = timer.elapsed();
                let limits = if elapsed < self.limits.time() / 2 {
                    Limits::Depth(d)
                } else if elapsed < self.limits.time() {
                    Limits::Time(self.limits.time() - elapsed)
                } else {
                    break;
                };

                yield block_in_place(|| self.strategy.search(&pos, limits));
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::search::{MockSearch, Pv};
    use futures_util::TryStreamExt;
    use mockall::predicate::eq;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn new_applies_default_search_limits() {
        assert_eq!(Ai::new(MockSearch::new()).limits, Limits::default());
    }

    #[proptest]
    fn play_finds_best_move(
        l: Limits,
        pos: Position,
        #[by_ref]
        #[filter(!#pv.is_empty())]
        pv: Pv,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy.expect_search().return_const(pv.clone());

        let mut ai = Ai::with_config(strategy, l);
        assert_eq!(rt.block_on(ai.play(&pos)).ok(), pv.iter().copied().next());
    }

    #[proptest]
    #[should_panic]
    fn play_panics_if_there_are_no_legal_moves(l: Limits, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();
        strategy.expect_search().return_const(Pv::default());

        let mut ai = Ai::with_config(strategy, l);
        rt.block_on(ai.play(&pos))?;
    }

    #[proptest]
    fn analyze_returns_sequence_of_principal_variations(pos: Position, pvs: Vec<Pv>) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = MockSearch::new();

        for (d, pv) in pvs.iter().enumerate() {
            strategy
                .expect_search()
                .with(eq(pos.clone()), eq(Limits::Depth((d + 1).try_into()?)))
                .return_const(pv.clone());
        }

        let mut ai = Ai::with_config(strategy, Limits::Depth(pvs.len().try_into()?));
        assert_eq!(
            rt.block_on(ai.analyze(&pos).try_collect::<Vec<_>>()),
            Ok(pvs)
        );
    }

    #[proptest]
    fn analyze_can_be_limited_by_time(pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let strategy = MockSearch::new();

        let mut ai = Ai::with_config(strategy, Limits::Time(Duration::ZERO));
        assert_eq!(
            rt.block_on(ai.analyze(&pos).try_collect::<Vec<_>>()),
            Ok(Vec::new())
        );
    }
}
