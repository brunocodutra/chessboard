use crate::{Action, Eval, Game, Search, SearchControl};
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

    fn negamax(&self, game: &Game, depth: u32, alpha: i32, beta: i32) -> (Option<Action>, i32) {
        debug_assert!(alpha < beta);

        if depth == 0 || game.outcome().is_some() {
            return (None, self.engine.eval(game));
        }

        let cutoff = AtomicI32::new(alpha);

        game.position()
            .moves()
            .par_bridge()
            .map(|m| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return None;
                }

                let mut game = game.clone();
                game.execute(m.into()).expect("expected legal move");

                let (_, s) = self.negamax(
                    &game,
                    depth - 1,
                    beta.saturating_neg(),
                    alpha.saturating_neg(),
                );

                cutoff.fetch_max(s.saturating_neg(), Ordering::Relaxed);

                Some((m.into(), s))
            })
            .while_some()
            .min_by_key(|&(_, s)| s)
            .map(|(m, s)| (Some(m), s.saturating_neg()))
            .unwrap_or_else(|| (None, self.engine.eval(game)))
    }
}

impl<E: Eval + Send + Sync> Search for Negamax<E> {
    fn search(&self, game: &Game, ctrl: SearchControl) -> Option<Action> {
        let max_depth = ctrl.max_depth.unwrap_or(Self::DEPTH);
        let (best, _) = self.negamax(game, max_depth, i32::MIN, i32::MAX);
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{engine::Random, MockEval, Outcome};
    use mockall::predicate::*;
    use std::iter::repeat;
    use test_strategy::proptest;

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn negamax_panics_if_alpha_not_smaller_than_beta(g: Game, a: i32, b: i32) {
        Negamax::new(MockEval::new()).negamax(&g, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn negamax_returns_none_if_depth_is_zero(g: Game, s: i32) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, 0, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_none_if_game_has_ended(
        _o: Outcome,
        #[any(Some(#_o))] g: Game,
        d: u32,
        s: i32,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, d, i32::MIN, i32::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_move_with_best_score(g: Game) {
        let engine = Random::new();

        let best = g
            .position()
            .moves()
            .zip(repeat(g.clone()))
            .map(|(m, mut g)| {
                g.execute(m.into()).unwrap();
                let s = g
                    .position()
                    .moves()
                    .zip(repeat(g.clone()))
                    .map(|(m, mut g)| {
                        g.execute(m.into()).unwrap();
                        engine.eval(&g).saturating_neg()
                    })
                    .max()
                    .unwrap_or_else(|| engine.eval(&g))
                    .saturating_neg();

                (Some(m.into()), s)
            })
            .max_by_key(|&(_, s)| s)
            .unwrap_or((None, engine.eval(&g)));

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, 2, i32::MIN, i32::MAX), best);
    }

    #[proptest]
    fn search_runs_negamax_with_max_depth(g: Game, #[strategy(0u32..=2)] d: u32) {
        let engine = Random::new();
        let ctrl = SearchControl { max_depth: Some(d) };
        let strategy = Negamax::new(engine);
        assert_eq!(
            strategy.search(&g, ctrl),
            strategy.negamax(&g, d, i32::MIN, i32::MAX).0
        );
    }
}
