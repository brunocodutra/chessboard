use crate::ai::Ai;
use futures_util::future::BoxFuture;
use lib::chess::{Move, Position};
use lib::search::{Limits, Options, Pv};
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
        Pv::new(pv.score(), pv.depth(), pv)
    }

    fn with_options(_: Options) -> Self {
        Self::new()
    }
}

#[cfg(test)]
type Strategy = MockSearcher;

#[cfg(not(test))]
type Strategy = lib::search::Searcher;

/// A chess engine.
#[derive(Debug, Default)]
pub struct Engine {
    strategy: Strategy,
}

impl Engine {
    /// Initializes the engine with the given search [`Options`].
    pub fn new(options: Options) -> Self {
        Engine {
            strategy: Strategy::with_options(options),
        }
    }
}

impl Ai for Engine {
    #[instrument(level = "debug", skip(self, pos), ret(Display), fields(%pos, depth, score))]
    fn play<'a, 'b, 'c>(&'a mut self, pos: &'b Position, limits: Limits) -> BoxFuture<'c, Move>
    where
        'a: 'c,
        'b: 'c,
    {
        Box::pin(async move {
            let pv: Pv<1> = block_in_place(|| self.strategy.search(pos, limits));

            Span::current()
                .record("depth", display(pv.depth()))
                .record("score", display(pv.score()));

            *pv.first().expect("expected some legal move")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib::search::{Depth, Score};
    use test_strategy::proptest;

    #[proptest(async = "tokio")]
    async fn play_finds_best_move(l: Limits, pos: Position, #[filter(!#pv.is_empty())] pv: Pv) {
        let mut strategy = Strategy::new();
        strategy.expect_search().return_const(pv.clone());

        let mut engine = Engine { strategy };
        assert_eq!(Some(engine.play(&pos, l).await), pv.first().copied());
    }

    #[proptest(async = "tokio")]
    #[should_panic]
    async fn play_panics_if_there_are_no_legal_moves(l: Limits, pos: Position, s: Score, d: Depth) {
        let mut strategy = Strategy::new();
        strategy.expect_search().return_const(Pv::new(s, d, []));

        let mut engine = Engine { strategy };
        engine.play(&pos, l).await;
    }
}
