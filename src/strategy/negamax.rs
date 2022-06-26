use crate::{Action, Eval, Game, Search};
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{fmt::Debug, str::FromStr};

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
#[serde(deny_unknown_fields, rename = "config", default)]
pub struct NegamaxConfig {
    #[cfg_attr(test, strategy(0i8..=2))]
    #[serde(deserialize_with = "deserialize_max_depth")]
    pub max_depth: i8,
}

fn deserialize_max_depth<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i8, D::Error> {
    match i8::deserialize(deserializer)? {
        d @ 0.. => Ok(d),
        d => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Unsigned(d as u64),
            &"a non negative number",
        )),
    }
}

impl Default for NegamaxConfig {
    fn default() -> Self {
        Self { max_depth: 5 }
    }
}

/// The reason why parsing [`NegamaxConfig`] failed.
#[derive(Debug, Display, PartialEq, Error, From)]
#[display(fmt = "failed to parse negamax configuration")]
pub struct ParseNegamaxConfigError(ron::de::Error);

impl FromStr for NegamaxConfig {
    type Err = ParseNegamaxConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[derive(Debug, Clone, From)]
pub struct Negamax<E: Eval + Send + Sync> {
    engine: E,
    config: NegamaxConfig,
}

impl<E: Eval + Send + Sync> Negamax<E> {
    /// Constructs [`Negamax`] with the default [`NegamaxConfig`].
    pub fn new(engine: E) -> Self {
        Self::with_config(engine, NegamaxConfig::default())
    }

    /// Constructs [`Negamax`] with the specified [`NegamaxConfig`].
    pub fn with_config(engine: E, config: NegamaxConfig) -> Self {
        Negamax { engine, config }
    }

    fn negamax(&self, game: &Game, height: i8, alpha: i16, beta: i16) -> (Option<Action>, i16) {
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
    fn search(&self, game: &Game) -> Option<Action> {
        let (best, _) = self.negamax(game, self.config.max_depth, i16::MIN, i16::MAX);
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
    fn config_deserializes_missing_fields_to_default() {
        assert_eq!("config()".parse(), Ok(NegamaxConfig::default()));
    }

    #[proptest]
    fn config_fails_to_deserialize_negative_max_depth(#[strategy(i8::MIN..0)] d: i8) {
        assert!(matches!(
            format!("config(max_depth:{})", d).parse::<NegamaxConfig>(),
            Err(ParseNegamaxConfigError(_))
        ));
    }

    #[proptest]
    fn parsing_printed_config_is_an_identity(c: NegamaxConfig) {
        assert_eq!(c.to_string().parse(), Ok(c));
    }

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
        d: i8,
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
    fn search_runs_negamax_with_max_depth(g: Game, cfg: NegamaxConfig) {
        let strategy = Negamax::with_config(Random::new(), cfg);
        assert_eq!(
            strategy.search(&g),
            strategy.negamax(&g, cfg.max_depth, i16::MIN, i16::MAX).0
        );
    }
}
