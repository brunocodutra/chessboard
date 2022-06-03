use crate::{Eval, Move, Position, Search, SearchControl};
use derive_more::{Constructor, From};
use rayon::prelude::*;
use std::fmt::Debug;
use std::sync::atomic::{AtomicI32, Ordering};

#[derive(Debug, Clone, From, Constructor)]
pub struct Negamax<E: Eval + Send + Sync> {
    engine: E,
}

impl<E: Eval + Send + Sync> Negamax<E> {
    const DEPTH: u32 = 5;

    fn negamax(&self, pos: &Position, depth: u32, alpha: i32, beta: i32) -> (Option<Move>, i32) {
        debug_assert!(alpha < beta);

        if depth == 0 {
            return (None, self.engine.eval(pos));
        }

        let cutoff = AtomicI32::new(alpha);

        pos.moves()
            .par_bridge()
            .map(|m| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return None;
                }

                let mut pos = pos.clone();
                pos.play(m).expect("expected legal move");

                let (_, s) = self.negamax(
                    &pos,
                    depth - 1,
                    beta.saturating_neg(),
                    alpha.saturating_neg(),
                );

                cutoff.fetch_max(s.saturating_neg(), Ordering::Relaxed);

                Some((m, s))
            })
            .while_some()
            .min_by_key(|&(_, s)| s)
            .map(|(m, s)| (Some(m), s.saturating_neg()))
            .unwrap_or_else(|| (None, self.engine.eval(pos)))
    }
}

impl<E: Eval + Send + Sync> Search for Negamax<E> {
    fn search(&self, pos: &Position, ctrl: SearchControl) -> Option<Move> {
        let max_depth = ctrl.max_depth.unwrap_or(Self::DEPTH);
        let (best, _) = self.negamax(pos, max_depth, i32::MIN, i32::MAX);
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::Random, MockEval, PositionKind};
    use mockall::predicate::*;
    use std::iter::repeat;
    use test_strategy::proptest;

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn negamax_panics_if_alpha_not_smaller_than_beta(pos: Position, a: i32, b: i32) {
        Negamax::new(MockEval::new()).negamax(&pos, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn negamax_returns_none_if_depth_is_zero(pos: Position, s: i32) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&pos, 0, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_none_if_there_are_no_moves(
        #[any(PositionKind::Stalemate)] pos: Position,
        d: u32,
        s: i32,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&pos, d, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_move_with_best_score(pos: Position) {
        let engine = Random::new();

        let best = pos
            .moves()
            .zip(repeat(pos.clone()))
            .map(|(m, mut pos)| {
                pos.play(m).unwrap();
                let s = pos
                    .moves()
                    .zip(repeat(pos.clone()))
                    .map(|(m, mut pos)| {
                        pos.play(m).unwrap();
                        engine.eval(&pos).saturating_neg()
                    })
                    .max()
                    .unwrap_or_else(|| engine.eval(&pos))
                    .saturating_neg();

                (Some(m), s)
            })
            .max_by_key(|&(_, s)| s)
            .unwrap_or((None, engine.eval(&pos)));

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&pos, 2, i32::MIN, i32::MAX), best);
    }

    #[proptest]
    fn search_runs_negamax_with_max_depth(pos: Position, #[strategy(0u32..=2)] d: u32) {
        let engine = Random::new();
        let ctrl = SearchControl { max_depth: Some(d) };
        let strategy = Negamax::new(engine);
        assert_eq!(
            strategy.search(&pos, ctrl),
            strategy.negamax(&pos, d, i32::MIN, i32::MAX).0
        );
    }
}
