use crate::{Eval, Move, Position, Search, Transposition, TranspositionTable};
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
        let table_size = 1 << 24;

        #[cfg(test)]
        #[cfg(tarpaulin)]
        let max_depth = 2;

        #[cfg(test)]
        #[cfg(not(tarpaulin))]
        let max_depth = 3;

        #[cfg(not(test))]
        let max_depth = 6;

        Self {
            max_depth,
            table_size,
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
    fn alpha_beta(&self, pos: &Position, draft: i8, mut alpha: i16, mut beta: i16) -> i16 {
        debug_assert!(alpha < beta, "{} < {}", alpha, beta);

        let zobrist = pos.zobrist();
        let transposition = self.tt.get(zobrist);

        if let Some(t) = transposition.filter(|t| t.draft() >= draft) {
            let (lower, upper) = t.bounds();
            (alpha, beta) = (alpha.max(lower), beta.min(upper));

            if alpha >= beta {
                return t.score();
            }
        }

        if draft <= 0 {
            return self.engine.eval(pos).max(-i16::MAX);
        }

        if let Some(m) = transposition.map(|t| t.best()) {
            let mut pos = pos.clone();
            if pos.play(m).is_ok() {
                let score = -self.alpha_beta(&pos, draft - 1, -beta, -alpha);
                alpha = alpha.max(score);
                if alpha >= beta {
                    return score;
                }
            }
        }

        let mut children: Vec<_> = pos.children().collect();
        children.par_sort_by_cached_key(|(_, pos)| self.alpha_beta(pos, 0, -beta, -alpha));

        if children.is_empty() {
            return self.engine.eval(pos).max(-i16::MAX);
        }

        let cutoff = AtomicI16::new(alpha);

        let (best, score) = children
            .into_par_iter()
            .filter_map(|(m, pos)| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return None;
                }

                let score = -self.alpha_beta(&pos, draft - 1, -beta, -alpha);
                cutoff.fetch_max(score, Ordering::Relaxed);
                Some((m, score))
            })
            .max_by_key(|(_, s)| *s)
            .expect("expected at least one legal move");

        let transposition = Transposition::new(score, alpha, beta, draft, best);
        self.tt.set(zobrist, transposition);

        score
    }

    /// The [mtd(f)] algorithm.
    ///
    /// [mtd(f)]: https://en.wikipedia.org/wiki/MTD(f)
    fn mtdf(&self, pos: &Position, depth: i8, mut score: i16) -> i16 {
        let mut alpha = -i16::MAX;
        let mut beta = i16::MAX;
        while alpha < beta {
            let target = score.max(alpha + 1);
            score = self.alpha_beta(pos, depth, target - 1, target);
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
    fn search(&self, pos: &Position) -> Option<Move> {
        let zobrist = pos.zobrist();
        let (mut score, depth) = match self.tt.get(zobrist) {
            Some(t) => (t.score(), t.draft()),
            _ => (self.engine.eval(pos), 1),
        };

        for d in depth..=self.config.max_depth.min(i8::MAX as u8) as i8 {
            score = self.mtdf(pos, d, score);
        }

        self.tt.get(zobrist).map(|t| t.best())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, MockEval};
    use mockall::predicate::*;
    use test_strategy::proptest;

    fn minimax<E: Eval + Sync>(engine: &E, pos: &Position, draft: i8) -> i16 {
        let children = pos.children();

        if draft == 0 || children.len() == 0 {
            return engine.eval(pos).max(-i16::MAX);
        }

        children
            .map(|(_, pos)| -minimax(engine, &pos, draft - 1))
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
    fn alpha_beta_panics_if_alpha_not_smaller_than_beta(pos: Position, a: i16, b: i16) {
        Minimax::new(MockEval::new()).alpha_beta(&pos, 0, a.max(b), a.min(b));
    }

    #[proptest]
    fn alpha_beta_returns_none_if_depth_is_zero(pos: Position, s: i16) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let strategy = Minimax::new(engine);
        assert_eq!(strategy.alpha_beta(&pos, 0, -i16::MAX, i16::MAX), s);
    }

    #[proptest]
    fn alpha_beta_returns_best_score(c: MinimaxConfig, pos: Position) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Engine::default(), &pos, depth),
            Minimax::with_config(Engine::default(), c).alpha_beta(&pos, depth, -i16::MAX, i16::MAX),
        );
    }

    #[proptest]
    fn alpha_beta_does_not_depend_on_table_size(
        #[strategy(0usize..65536)] a: usize,
        #[strategy(0usize..65536)] b: usize,
        c: MinimaxConfig,
        pos: Position,
    ) {
        let a = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: a, ..c });
        let b = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: b, ..c });

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.alpha_beta(&pos, depth, -i16::MAX, i16::MAX),
            b.alpha_beta(&pos, depth, -i16::MAX, i16::MAX)
        );
    }

    #[proptest]
    fn mtdf_returns_best_score(c: MinimaxConfig, pos: Position) {
        let depth = c.max_depth.try_into()?;

        assert_eq!(
            minimax(&Engine::default(), &pos, depth),
            Minimax::with_config(Engine::default(), c).mtdf(&pos, depth, 0),
        );
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(c: MinimaxConfig, pos: Position, s: i16) {
        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        let depth = c.max_depth.try_into()?;

        assert_eq!(a.mtdf(&pos, depth, s), b.mtdf(&pos, depth, 0));
    }

    #[proptest]
    fn mtdf_is_equivalent_to_alpha_beta(c: MinimaxConfig, pos: Position) {
        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        let depth = c.max_depth.try_into()?;

        assert_eq!(
            a.mtdf(&pos, depth, 0),
            b.alpha_beta(&pos, depth, -i16::MAX, i16::MAX),
        );
    }

    #[proptest]
    fn search_finds_the_best_move(c: MinimaxConfig, pos: Position) {
        let strategy = Minimax::with_config(Engine::default(), c);

        assert_eq!(
            strategy.search(&pos),
            strategy.tt.get(pos.zobrist()).map(|t| t.best())
        );
    }

    #[proptest]
    fn search_is_stable(c: MinimaxConfig, pos: Position) {
        let strategy = Minimax::with_config(Engine::default(), c);
        assert_eq!(strategy.search(&pos), strategy.search(&pos));
    }
}
