use super::Player;
use async_stream::try_stream;
use futures_util::{future::BoxFuture, stream::BoxStream};
use lib::chess::{Move, Position};
use lib::eval::Evaluator;
use lib::search::{Depth, Limits, Options, Pv};
use std::{convert::Infallible, time::Instant};
use tokio::task::block_in_place;
use tracing::{field::display, instrument, Span};

#[cfg(test)]
#[mockall::automock]
trait Searcher {
    fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<4>;
}

#[cfg(test)]
impl MockSearcher {
    fn with_options(_: Evaluator, _: Options) -> Self {
        Self::new()
    }
}

#[cfg(test)]
type Strategy = MockSearcher;

#[cfg(not(test))]
type Strategy = lib::search::Searcher;

/// A chess engine.
#[derive(Debug, Default)]
pub struct Ai {
    strategy: Strategy,
}

impl Ai {
    /// Constructs [`Ai`] with the given [`Options`].
    pub fn new(evaluator: Evaluator, options: Options) -> Self {
        Ai {
            strategy: Strategy::with_options(evaluator, options),
        }
    }
}

impl Player for Ai {
    type Error = Infallible;

    #[instrument(level = "debug", skip(self, pos), ret(Display), err, fields(%pos, %limits, depth, score))]
    fn play<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxFuture<'c, Result<Move, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(async move {
            let pv = block_in_place(|| self.strategy.search::<1>(pos, limits));

            if let Some((d, s)) = Option::zip(pv.depth(), pv.score()) {
                Span::current()
                    .record("depth", display(d))
                    .record("score", display(s));
            }

            Ok(*pv.first().expect("expected at least one legal move"))
        })
    }

    #[instrument(level = "debug", skip(self, pos), fields(%pos, %limits))]
    fn analyze<'a, 'b, 'c, const N: usize>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxStream<'c, Result<Pv<N>, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(try_stream! {
            let timer = Instant::now();
            for d in 1..=limits.depth().get() {
                let elapsed = timer.elapsed();
                let limits = if elapsed < limits.time() / 2 {
                    Depth::new(d).into()
                } else if elapsed < limits.time() {
                    Limits::Time(limits.time() - elapsed)
                } else {
                    break;
                };

                yield block_in_place(|| self.strategy.search::<N>(pos, limits).truncate());
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::TryStreamExt;
    use lib::search::Pv;
    use mockall::predicate::eq;
    use proptest::sample::size_range;
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn play_finds_best_move(l: Limits, pos: Position, #[filter(!#pv.is_empty())] pv: Pv<4>) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();
        strategy.expect_search().return_const(pv.clone());

        let mut ai = Ai { strategy };

        assert_eq!(
            rt.block_on(ai.play(&pos, l)).ok(),
            pv.iter().copied().next()
        );
    }

    #[proptest]
    #[should_panic]
    fn play_panics_if_there_are_no_legal_moves(l: Limits, pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();
        strategy.expect_search().return_const(Pv::default());

        let mut ai = Ai { strategy };
        rt.block_on(ai.play(&pos, l))?;
    }

    #[proptest]
    fn analyze_returns_sequence_of_principal_variations(
        pos: Position,
        #[any(size_range(1..=3).lift())] pvs: Vec<Pv<4>>,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();

        for (d, pv) in pvs.iter().enumerate() {
            strategy
                .expect_search()
                .with(eq(pos.clone()), eq(Limits::Depth(Depth::saturate(d + 1))))
                .return_const(pv.clone());
        }

        let mut ai = Ai { strategy };
        let l = Limits::Depth(Depth::saturate(pvs.len()));

        assert_eq!(
            rt.block_on(ai.analyze(&pos, l).try_collect::<Vec<_>>()),
            Ok(pvs)
        );
    }

    #[proptest]
    fn analyze_can_be_limited_by_time(pos: Position) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut ai = Ai {
            strategy: Strategy::new(),
        };

        let l = Limits::Time(Duration::ZERO);

        assert_eq!(
            rt.block_on(ai.analyze::<4>(&pos, l).try_collect::<Vec<_>>()),
            Ok(Vec::new())
        );
    }
}
