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

        let mvs = pos.moves();

        if depth == 0 || mvs.len() == 0 {
            return (None, self.engine.evaluate(&pos));
        }

        let mut best = None;
        let mut score = i32::MIN;

        for mv in mvs {
            let mut next = pos.clone();
            next.play(mv).expect("expected legal move");

            let (_, s) = self.negamax(next, depth - 1, b.saturating_neg(), a.saturating_neg());

            if score < s.saturating_neg() {
                score = s.saturating_neg();
                best = Some(mv);
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
    use crate::Checkmate;
    use mockall::predicate::*;
    use proptest::prelude::*;
    use smol::block_on;

    proptest! {
        #[test]
        #[should_panic]
        #[cfg(debug_assertions)]
        fn negamax_panics_if_alpha_not_smaller_than_beta(pos: Position, a: i32, b: i32) {
            Negamax::new(MockEngine::new()).negamax(pos, 0, a.max(b), a.min(b));
        }

        #[test]
        fn negamax_returns_none_if_depth_is_zero(pos: Position, s: i32) {
            let mut engine = MockEngine::new();
            engine.expect_evaluate().times(1).with(eq(pos.clone())).returning(move |_| s);

            let strategy = Negamax::new(engine);
            assert_eq!(strategy.negamax(pos, 0, i32::MIN, i32::MAX), (None, s));
        }

        #[test]
        fn negamax_returns_none_if_there_are_no_moves(pos: Checkmate, d: u32, s: i32) {
            let mut engine = MockEngine::new();
            engine.expect_evaluate().times(1).with(eq(pos.clone())).returning(move |_| s);

            let strategy = Negamax::new(engine);
            assert_eq!(strategy.negamax(pos.into(), d, i32::MIN, i32::MAX), (None, s));
        }

        #[test]
        fn negamax_returns_move_with_best_score(pos: Position, mut s: i8) {
            let mvs = pos.moves();
            prop_assume!(mvs.len() > 0);

            let mut engine = MockEngine::new();

            let mut score: i32 = s.into();
            let ps = mvs.clone().map(|m| { let mut p = pos.clone(); p.play(m).unwrap(); p } );
            engine.expect_evaluate().times(mvs.len())
                .with(in_hash(ps))
                .returning(move |_| { score -= 1; score });

            let score: i32 = mvs.len() as i32 - score;
            let strategy = Negamax::new(engine);
            assert_eq!(strategy.negamax(pos, 1, i32::MIN, i32::MAX), (mvs.last(), score));
        }

        #[test]
        fn search_runs_negamax(pos: Position) {
            if let Some(mv) = block_on(Negamax::new(Random).search(&pos)) {
                assert!(pos.moves().any(|m| m == mv));
            } else {
                assert_eq!(pos.moves().len(), 0);
            }
        }
    }
}
