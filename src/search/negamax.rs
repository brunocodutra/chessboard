use crate::{Engine, Move, Position, Search};
use async_trait::async_trait;
use derive_more::Constructor;
use smol::unblock;
use std::fmt::Debug;
use tracing::instrument;

#[derive(Debug, Clone, Constructor)]
pub struct Negamax<E: Engine> {
    engine: E,
}

impl<E: Engine> Negamax<E> {
    #[cfg(debug_assertions)]
    const DEPTH: u32 = 2;

    #[cfg(not(debug_assertions))]
    const DEPTH: u32 = 4;

    #[cfg_attr(debug_assertions, instrument(level = "trace", skip(self)))]
    fn negamax(&self, pos: Position, depth: u32, mut a: i32, b: i32) -> (Option<Move>, i32) {
        debug_assert!(a < b);

        let moves = pos.moves();

        if depth == 0 || moves.len() == 0 {
            return (None, self.engine.evaluate(&pos));
        }

        let mut best = None;
        let mut score = i32::MIN;

        for m in moves {
            let mut next = pos.clone();
            next.play(m).expect("expected legal move");

            let (_, s) = self.negamax(next, depth - 1, b.saturating_neg(), a.saturating_neg());

            if score < s.saturating_neg() {
                score = s.saturating_neg();
                best = Some(m);
                a = a.max(score);
            }

            #[cfg(debug_assertions)]
            tracing::debug!(%score, alpha = %a, beta = %b);

            if a >= b {
                break;
            }
        }

        #[cfg(debug_assertions)]
        tracing::debug!(?best, %score);

        (best, score)
    }
}

#[async_trait]
impl<E: Engine + Debug + Clone + Send + 'static> Search for Negamax<E> {
    #[instrument(level = "trace")]
    async fn search(&mut self, pos: &Position) -> Option<Move> {
        let pos = pos.clone();
        let this = self.clone();
        let (best, _) = unblock(move || this.negamax(pos, Self::DEPTH, i32::MIN, i32::MAX)).await;
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{MockEngine, Random};
    use crate::PositionKind;
    use mockall::predicate::*;
    use smol::block_on;
    use test_strategy::proptest;

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn negamax_panics_if_alpha_not_smaller_than_beta(pos: Position, a: i32, b: i32) {
        Negamax::new(MockEngine::new()).negamax(pos, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn negamax_returns_none_if_depth_is_zero(pos: Position, s: i32) {
        let mut engine = MockEngine::new();
        engine
            .expect_evaluate()
            .times(1)
            .with(eq(pos.clone()))
            .returning(move |_| s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(pos, 0, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_none_if_there_are_no_moves(
        #[any(PositionKind::Checkmate)] pos: Position,
        d: u32,
        s: i32,
    ) {
        let mut engine = MockEngine::new();
        engine
            .expect_evaluate()
            .times(1)
            .with(eq(pos.clone()))
            .returning(move |_| s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(pos, d, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_move_with_best_score(
        #[by_ref]
        #[filter(#pos.moves().len() > 0)]
        pos: Position,
        #[strategy(-128..=128)] mut score: i32,
    ) {
        let moves = pos.moves();
        let positions = moves.clone().map(|m| {
            let mut pos = pos.clone();
            pos.play(m).unwrap();
            pos
        });

        let mut engine = MockEngine::new();

        engine
            .expect_evaluate()
            .times(moves.len())
            .with(in_hash(positions))
            .returning(move |_| {
                score -= 1;
                score
            });

        let score: i32 = moves.len() as i32 - score;
        let strategy = Negamax::new(engine);
        assert_eq!(
            strategy.negamax(pos, 1, i32::MIN, i32::MAX),
            (moves.last(), score)
        );
    }

    #[proptest]
    fn search_runs_negamax(pos: Position) {
        if let Some(m) = block_on(Negamax::new(Random).search(&pos)) {
            assert!(pos.moves().any(|n| n == m));
        } else {
            assert_eq!(pos.moves().len(), 0);
        }
    }
}
