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
    fn search(&mut self, pos: &Position, limits: Limits) -> Pv;
}

#[cfg(test)]
impl MockSearcher {
    fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let pv = Searcher::search(self, pos, limits);
        Pv::new(pv.depth(), pv.score(), pv.iter().copied().collect())
    }

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
            let pv: Pv<1> = block_in_place(|| self.strategy.search(pos, limits));

            Span::current()
                .record("depth", display(pv.depth()))
                .record("score", display(pv.score()));

            Ok(*pv.first().expect("expected some legal move"))
        })
    }

    #[instrument(level = "debug", skip(self, pos), fields(%pos, %limits))]
    fn analyze<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxStream<'c, Result<Pv, Self::Error>>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(try_stream! {
            let timer = Instant::now();
            for d in 1..=limits.depth().get() {
                let elapsed = timer.elapsed();
                if elapsed < limits.time() / 2 {
                    let depth = Depth::new(d);
                    yield block_in_place(|| self.strategy.search(pos, depth.into()));
                } else if elapsed < limits.time() {
                    let time = limits.time() - elapsed;
                    yield block_in_place(|| self.strategy.search(pos, time.into()));
                    break;
                } else {
                    break;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::TryStreamExt;
    use lib::search::{Line, Score};
    use mockall::predicate::eq;
    use proptest::sample::size_range;
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn play_finds_best_move(l: Limits, pos: Position, #[filter(!#pv.is_empty())] pv: Pv) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();
        strategy.expect_search().return_const(pv.clone());

        let mut ai = Ai { strategy };
        assert_eq!(rt.block_on(ai.play(&pos, l)).ok(), pv.first().copied());
    }

    #[proptest]
    #[should_panic]
    fn play_panics_if_there_are_no_legal_moves(l: Limits, pos: Position, d: Depth, s: Score) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();
        strategy
            .expect_search()
            .return_const(Pv::new(d, s, Line::default()));

        let mut ai = Ai { strategy };
        rt.block_on(ai.play(&pos, l))?;
    }

    #[proptest]
    fn analyze_returns_sequence_of_principal_variations(
        pos: Position,
        #[any(size_range(0..=3).lift())] pvs: Vec<Pv>,
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
            rt.block_on(ai.analyze(&pos, l).try_collect::<Vec<_>>()),
            Ok(Vec::new())
        );
    }
}
