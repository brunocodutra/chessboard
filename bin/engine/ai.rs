use super::Player;
use async_stream::try_stream;
use futures_util::{future::BoxFuture, stream::BoxStream};
use lib::chess::{Move, Position};
use lib::eval::Evaluator;
use lib::search::{Depth, Limits, Options, Report};
use std::{convert::Infallible, time::Instant};
use tokio::task::block_in_place;
use tracing::{field::display, instrument, Span};

#[cfg(test)]
#[mockall::automock]
trait Searcher {
    fn search(&mut self, pos: &Position, limits: Limits) -> Report;
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
            let report = block_in_place(|| self.strategy.search(pos, limits));

            Span::current()
                .record("depth", display(report.depth()))
                .record("score", display(report.score()));

            Ok(*report.pv().first().expect("expected some legal move"))
        })
    }

    #[instrument(level = "debug", skip(self, pos), fields(%pos, %limits))]
    fn analyze<'a, 'b, 'c>(
        &'a mut self,
        pos: &'b Position,
        limits: Limits,
    ) -> BoxStream<'c, Result<Report, Self::Error>>
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
    use lib::search::{Pv, Score};
    use mockall::predicate::eq;
    use proptest::sample::size_range;
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn play_finds_best_move(l: Limits, pos: Position, #[filter(!#r.pv().is_empty())] r: Report) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();
        strategy.expect_search().return_const(r.clone());

        let mut ai = Ai { strategy };

        assert_eq!(
            rt.block_on(ai.play(&pos, l)).ok(),
            r.pv().iter().copied().next()
        );
    }

    #[proptest]
    #[should_panic]
    fn play_panics_if_there_are_no_legal_moves(l: Limits, pos: Position, d: Depth, s: Score) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();
        strategy
            .expect_search()
            .return_const(Report::new(d, s, Pv::default()));

        let mut ai = Ai { strategy };
        rt.block_on(ai.play(&pos, l))?;
    }

    #[proptest]
    fn analyze_returns_sequence_of_search_reports(
        pos: Position,
        #[any(size_range(0..=3).lift())] rs: Vec<Report>,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let mut strategy = Strategy::new();

        for (d, r) in rs.iter().enumerate() {
            strategy
                .expect_search()
                .with(eq(pos.clone()), eq(Limits::Depth(Depth::saturate(d + 1))))
                .return_const(r.clone());
        }

        let mut ai = Ai { strategy };
        let l = Limits::Depth(Depth::saturate(rs.len()));

        assert_eq!(
            rt.block_on(ai.analyze(&pos, l).try_collect::<Vec<_>>()),
            Ok(rs)
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
