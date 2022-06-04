use crate::{Eval, Move, Position, Search, SearchControl};
use derive_more::{Constructor, From};
use std::fmt::Debug;

#[derive(Debug, Clone, From, Constructor)]
pub struct Negamax<E: Eval> {
    engine: E,
}

impl<E: Eval> Negamax<E> {
    #[cfg(debug_assertions)]
    const DEPTH: u32 = 2;

    #[cfg(not(debug_assertions))]
    const DEPTH: u32 = 4;

    fn negamax(&self, pos: Position, depth: u32, mut a: i32, b: i32) -> (Option<Move>, i32) {
        debug_assert!(a < b);

        let moves = pos.moves();

        if depth == 0 || moves.len() == 0 {
            return (None, self.engine.eval(&pos));
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

impl<E: Eval> Search for Negamax<E> {
    fn search(&self, pos: &Position, ctrl: SearchControl) -> Option<Move> {
        let max_depth = ctrl.max_depth.unwrap_or(Self::DEPTH);
        let (best, _) = self.negamax(pos.clone(), max_depth, i32::MIN, i32::MAX);
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::Random, MockEval, PositionKind};
    use mockall::predicate::*;
    use test_strategy::proptest;

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn negamax_panics_if_alpha_not_smaller_than_beta(pos: Position, a: i32, b: i32) {
        Negamax::new(MockEval::new()).negamax(pos, 0, a.max(b), a.min(b));
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
        assert_eq!(strategy.negamax(pos, 0, i32::MIN, i32::MAX), (None, s));
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
        assert_eq!(strategy.negamax(pos, d, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_move_with_best_score(pos: Position) {
        let engine = Random::new();

        let best = pos
            .moves()
            .map(|m| {
                let mut pos = pos.clone();
                pos.play(m).unwrap();
                (m, engine.eval(&pos))
            })
            .min_by_key(|&(_, s)| s)
            .map(|(m, s)| (Some(m), s.saturating_neg()))
            .unwrap_or((None, engine.eval(&pos)));

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(pos, 1, i32::MIN, i32::MAX), best);
    }

    #[proptest]
    fn search_runs_negamax_with_max_depth(pos: Position, #[strategy(0u32..4)] d: u32) {
        let engine = Random::new();
        let ctrl = SearchControl { max_depth: Some(d) };
        let strategy = Negamax::new(engine);
        assert_eq!(
            strategy.search(&pos, ctrl),
            strategy.negamax(pos, d, i32::MIN, i32::MAX).0
        );
    }
}
