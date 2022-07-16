use crate::{Action, Eval, Game, Search, Transposition, TranspositionTable};
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{fmt::Debug, str::FromStr};

/// Configuration for [`Minimax`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
#[serde(deny_unknown_fields, rename = "config", default)]
pub struct MinimaxConfig {
    /// The maximum number of plies to search.
    ///
    /// This is an upper limit, the actual depth searched may be smaller.
    #[cfg_attr(test, strategy(0u8..=MinimaxConfig::default().max_depth))]
    pub max_depth: u8,

    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[cfg_attr(test, strategy(8usize..=MinimaxConfig::default().table_size))]
    pub table_size: usize,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
        #[cfg(test)]
        #[cfg(tarpaulin)]
        {
            Self {
                max_depth: 2,
                table_size: 1 << 8,
            }
        }

        #[cfg(test)]
        #[cfg(not(tarpaulin))]
        {
            Self {
                max_depth: 3,
                table_size: 1 << 16,
            }
        }

        #[cfg(not(test))]
        {
            Self {
                max_depth: 6,
                table_size: 1 << 32,
            }
        }
    }
}

/// The reason why parsing [`MinimaxConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse minimax configuration")]
pub struct ParseMinimaxConfigError(ron::de::Error);

impl FromStr for MinimaxConfig {
    type Err = ParseMinimaxConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

/// An implementation of [minimax].
///
/// [minimax]: https://en.wikipedia.org/wiki/Minimax
#[derive(Debug)]
pub struct Minimax<E: Eval + Send + Sync> {
    engine: E,
    config: MinimaxConfig,
    tt: TranspositionTable,
}

impl<E: Eval + Send + Sync> Minimax<E> {
    /// Constructs [`Minimax`] with the default [`MinimaxConfig`].
    pub fn new(engine: E) -> Self {
        Self::with_config(engine, MinimaxConfig::default())
    }

    /// Constructs [`Minimax`] with the specified [`MinimaxConfig`].
    pub fn with_config(engine: E, config: MinimaxConfig) -> Self {
        Minimax {
            engine,
            config,
            tt: TranspositionTable::new(config.table_size),
        }
    }

    /// The [alpha-beta pruning] algorithm.
    ///
    /// [alpha-beta pruning]: https://en.wikipedia.org/wiki/Alpha%E2%80%93beta_pruning
    fn alpha_beta(&self, game: &Game, draft: i8, mut alpha: i16, mut beta: i16) -> i16 {
        debug_assert!(alpha < beta, "{} < {}", alpha, beta);

        let zobrist = game.position().zobrist();
        let transposition = self.tt.get(zobrist);

        if let Some(t) = transposition.filter(|t| t.draft() >= draft) {
            let (lower, upper) = t.bounds();
            (alpha, beta) = (alpha.max(lower), beta.min(upper));

            if alpha >= beta {
                return t.score();
            }
        }

        if draft <= 0 || game.outcome().is_some() {
            return self.engine.eval(game);
        }

        let pilot = transposition.and_then(|t| {
            let mut game = game.clone();
            game.execute(t.action()).ok()?;

            let score = self
                .alpha_beta(
                    &game,
                    draft - 1,
                    beta.saturating_neg(),
                    alpha.saturating_neg(),
                )
                .saturating_neg();

            alpha = alpha.max(score);
            Some((t.action(), score))
        });

        if alpha >= beta {
            return alpha;
        }

        let mut successors: Vec<_> = game
            .actions()
            .par_bridge()
            .filter(|a| Some(*a) != pilot.map(|(pv, _)| pv))
            .map(|action| {
                let mut game = game.clone();
                game.execute(action).expect("expected legal action");

                let ordering =
                    self.alpha_beta(&game, 0, beta.saturating_neg(), alpha.saturating_neg());

                (action, game, ordering)
            })
            .collect();

        successors.par_sort_unstable_by_key(|(_, _, o)| *o);

        let cutoff = AtomicI16::new(alpha);

        let (action, score) = successors
            .into_par_iter()
            .filter_map(|(action, game, _)| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return None;
                }

                let score = self
                    .alpha_beta(
                        &game,
                        draft - 1,
                        beta.saturating_neg(),
                        alpha.saturating_neg(),
                    )
                    .saturating_neg();

                cutoff.fetch_max(score, Ordering::Relaxed);

                Some((action, score))
            })
            .chain(pilot)
            .max_by_key(|(_, s)| *s)
            .expect("expected at least one legal action");

        let transposition = Transposition::new(score, alpha, beta, draft, action);
        self.tt.set(zobrist, transposition);

        score
    }

    /// The [mtd(f)] algorithm.
    ///
    /// [mtd(f)]: https://en.wikipedia.org/wiki/MTD(f)
    fn mtdf(&self, game: &Game, depth: i8, mut score: i16) -> i16 {
        let mut alpha = -i16::MAX;
        let mut beta = i16::MAX;
        while alpha < beta {
            let target = score.max(alpha + 1);
            score = self.alpha_beta(game, depth, target - 1, target);
            if score < target {
                beta = score;
            } else {
                alpha = score;
            }
        }

        score
    }
}

impl<E: Eval + Send + Sync> Search for Minimax<E> {
    fn search(&self, game: &Game) -> Option<Action> {
        let zobrist = game.position().zobrist();
        let (mut score, depth) = match self.tt.get(zobrist) {
            Some(t) => (t.score(), t.draft()),
            _ => (self.engine.eval(game), 1),
        };

        for d in depth..=self.config.max_depth.min(i8::MAX as u8) as i8 {
            score = self.mtdf(game, d, score);
        }

        self.tt.get(zobrist).map(|t| t.action())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, MockEval, Outcome};
    use mockall::predicate::*;
    use test_strategy::proptest;

    fn minimax<E: Eval + Sync>(engine: &E, game: &Game, draft: i8) -> i16 {
        if draft == 0 || game.outcome().is_some() {
            return engine.eval(game);
        }

        game.actions()
            .par_bridge()
            .map(|a| {
                let mut game = game.clone();
                game.execute(a).unwrap();
                minimax(engine, &game, draft - 1).saturating_neg()
            })
            .max()
            .unwrap()
    }

    #[proptest]
    fn config_deserializes_missing_fields_to_default() {
        assert_eq!("config()".parse(), Ok(MinimaxConfig::default()));
    }

    #[proptest]
    fn parsing_printed_config_is_an_identity(c: MinimaxConfig) {
        assert_eq!(c.to_string().parse(), Ok(c));
    }

    #[proptest]
    fn table_size_is_an_upper_limit(c: MinimaxConfig) {
        let strategy = Minimax::with_config(MockEval::new(), c);
        assert!(strategy.tt.size() <= c.table_size);
    }

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn alpha_beta_panics_if_alpha_not_smaller_than_beta(g: Game, a: i16, b: i16) {
        Minimax::new(MockEval::new()).alpha_beta(&g, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn alpha_beta_returns_none_if_depth_is_zero(g: Game, s: i16) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Minimax::new(engine);
        assert_eq!(strategy.alpha_beta(&g, 0, i16::MIN, i16::MAX), s);
    }

    #[proptest]
    fn alpha_beta_returns_none_if_game_has_ended(
        _o: Outcome,
        #[any(Some(#_o))] g: Game,
        d: i8,
        s: i16,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(g.clone()))
            .return_const(s);

        let strategy = Minimax::new(engine);
        assert_eq!(strategy.alpha_beta(&g, d, i16::MIN, i16::MAX), s);
    }

    #[proptest]
    fn alpha_beta_returns_best_score(c: MinimaxConfig, g: Game) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Engine::default(), &g, depth),
            Minimax::with_config(Engine::default(), c).alpha_beta(&g, depth, i16::MIN, i16::MAX),
        );
    }

    #[proptest]
    fn alpha_beta_does_not_depend_on_table_size(
        #[strategy(0usize..65536)] a: usize,
        #[strategy(0usize..65536)] b: usize,
        c: MinimaxConfig,
        g: Game,
    ) {
        let a = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: a, ..c });
        let b = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: b, ..c });

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.alpha_beta(&g, depth, i16::MIN, i16::MAX),
            b.alpha_beta(&g, depth, i16::MIN, i16::MAX)
        );
    }

    #[proptest]
    fn mtdf_returns_best_score(c: MinimaxConfig, g: Game) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Engine::default(), &g, depth),
            Minimax::with_config(Engine::default(), c).mtdf(&g, depth, 0),
        );
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(c: MinimaxConfig, g: Game, s: i16) {
        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        let depth = c.max_depth.try_into()?;

        assert_eq!(a.mtdf(&g, depth, s), b.mtdf(&g, depth, 0));
    }

    #[proptest]
    fn mtdf_is_equivalent_to_alphabeta(c: MinimaxConfig, g: Game) {
        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.mtdf(&g, depth, 0),
            b.alpha_beta(&g, depth, i16::MIN, i16::MAX),
        );
    }

    #[proptest]
    fn search_finds_the_best_action(c: MinimaxConfig, g: Game) {
        let strategy = Minimax::with_config(Engine::default(), c);
        let zobrist = g.position().zobrist();
        assert_eq!(
            strategy.search(&g),
            strategy.tt.get(zobrist).map(|r| r.action())
        );
    }

    #[proptest]
    fn search_is_stable(c: MinimaxConfig, g: Game) {
        let strategy = Minimax::with_config(Engine::default(), c);
        assert_eq!(strategy.search(&g), strategy.search(&g));
    }
}
