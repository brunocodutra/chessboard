use crate::{Action, Eval, Game, Search, SearchControl};
use derive_more::{Constructor, From};
use rayon::prelude::*;
use std::fmt::Debug;
use std::sync::atomic::{AtomicI16, Ordering};

#[derive(Debug, Clone, From, Constructor)]
pub struct Negamax<E: Eval + Send + Sync> {
    engine: E,
}

impl<E: Eval + Send + Sync> Negamax<E> {
    const DEPTH: u8 = 5;

    fn negamax(&self, game: &Game, height: u8, alpha: i16, beta: i16) -> (Option<Action>, i16) {
        debug_assert!(alpha < beta);

        if height == 0 || game.outcome().is_some() {
            return (None, self.engine.eval(game));
        }

        let cutoff = AtomicI16::new(alpha);

        game.actions()
            .par_bridge()
            .map(|a| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return None;
                }

                let mut game = game.clone();
                game.execute(a).expect("expected legal action");

                let (_, s) = self.negamax(
                    &game,
                    height - 1,
                    beta.saturating_neg(),
                    alpha.saturating_neg(),
                );

                cutoff.fetch_max(s.saturating_neg(), Ordering::Relaxed);

                Some((Some(a), s.saturating_neg()))
            })
            .while_some()
            .max_by_key(|&(_, s)| s)
            .expect("expected at least one legal action")
    }
}

impl<E: Eval + Send + Sync> Search for Negamax<E> {
    fn search(&self, game: &Game, ctrl: SearchControl) -> Option<Action> {
        let depth = ctrl.depth.unwrap_or(Self::DEPTH);
        let (best, _) = self.negamax(game, depth, i16::MIN, i16::MAX);
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
    fn negamax_panics_if_alpha_not_smaller_than_beta(g: Game, a: i16, b: i16) {
        Negamax::new(MockEval::new()).negamax(&g, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn negamax_returns_none_if_depth_is_zero(g: Game, s: i16) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, 0, i16::MIN, i16::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_none_if_game_has_ended(
        _o: Outcome,
        #[any(Some(#_o))] g: Game,
        d: u8,
        s: i16,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, d, i16::MIN, i16::MAX), (None, s));
    }

    #[proptest]
    fn negamax_returns_move_with_best_score(g: Game) {
        let engine = Random::new();

        let best = g
            .actions()
            .zip(repeat(g.clone()))
            .map(|(a, mut g)| {
                g.execute(a).unwrap();
                let s = g
                    .actions()
                    .zip(repeat(g.clone()))
                    .map(|(a, mut g)| {
                        g.execute(a).unwrap();
                        engine.eval(&g).saturating_neg()
                    })
                    .max()
                    .unwrap_or_else(|| engine.eval(&g))
                    .saturating_neg();

                (Some(a), s)
            })
            .max_by_key(|&(_, s)| s)
            .unwrap_or((None, engine.eval(&g)));

        let strategy = Negamax::new(engine);
        assert_eq!(strategy.negamax(&g, 2, i16::MIN, i16::MAX), best);
    }

    #[proptest]
    fn search_runs_negamax_with_max_depth(g: Game, #[strategy(0u8..=2)] d: u8) {
        let engine = Random::new();
        let ctrl = SearchControl { depth: Some(d) };
        let strategy = Negamax::new(engine);
        assert_eq!(
            strategy.search(&g, ctrl),
            strategy.negamax(&g, d, i16::MIN, i16::MAX).0
        );
    }
}
