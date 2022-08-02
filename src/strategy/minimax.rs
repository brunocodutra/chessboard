use crate::{Eval, Move, Position, Search, Transposition, TranspositionTable};
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI16, Ordering};
use std::time::{Duration, Instant};
use std::{cmp::max_by_key, fmt::Debug, str::FromStr};

#[derive(Debug, Display, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display(fmt = "time is up!")]
pub struct Timeout;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
struct Timer {
    deadline: Option<Instant>,
}

impl Timer {
    fn start(duration: Duration) -> Self {
        Timer {
            deadline: Instant::now().checked_add(duration),
        }
    }

    fn elapsed(&self) -> Result<(), Timeout> {
        if self.deadline.map(|t| t.elapsed()) > Some(Duration::ZERO) {
            Err(Timeout)
        } else {
            Ok(())
        }
    }
}

/// Configuration for [`Minimax`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
#[serde(deny_unknown_fields, rename = "config", default)]
pub struct MinimaxConfig {
    /// The maximum number of plies to search.
    ///
    /// This is an upper limit, the actual depth searched may be smaller.
    #[cfg_attr(test, strategy(0u8..=MinimaxConfig::default().max_depth))]
    pub max_depth: u8,

    /// The maximum amount of time to spend searching.
    ///
    /// This is an upper limit, the actual time searched may be smaller.
    #[serde(with = "humantime_serde")]
    pub max_time: Duration,

    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[cfg_attr(test, strategy(0usize..=MinimaxConfig::default().table_size))]
    pub table_size: usize,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
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
            max_time: Duration::MAX,
            table_size: 1 << 24,
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
    fn alpha_beta(
        &self,
        pos: &Position,
        draft: i8,
        timer: Timer,
        mut alpha: i16,
        mut beta: i16,
    ) -> Result<i16, Timeout> {
        debug_assert!(alpha < beta, "{} < {}", alpha, beta);

        timer.elapsed()?;

        let zobrist = pos.zobrist();
        let transposition = self.tt.get(zobrist);

        if let Some(t) = transposition.filter(|t| t.draft() >= draft) {
            let (lower, upper) = t.bounds();
            (alpha, beta) = (alpha.max(lower), beta.min(upper));

            if alpha >= beta {
                return Ok(t.score());
            }
        }

        if draft <= 0 {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        }

        if let Some(m) = transposition.map(|t| t.best()) {
            let mut pos = pos.clone();
            if pos.play(m).is_ok() {
                let score = -self.alpha_beta(&pos, draft - 1, timer, -beta, -alpha)?;
                alpha = alpha.max(score);
                if alpha >= beta {
                    let transposition = Transposition::lower(score, draft, m);
                    self.tt.set(zobrist, transposition);
                    return Ok(score);
                }
            }
        }

        let mut children: Vec<_> = pos.children().collect();
        children.par_sort_by_cached_key(|(_, pos)| self.alpha_beta(pos, 0, timer, -beta, -alpha));

        if children.is_empty() {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        }

        let cutoff = AtomicI16::new(alpha);

        let (best, score) = children
            .into_par_iter()
            .map(|(m, pos)| {
                let alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return Ok(None);
                }

                let score = -self.alpha_beta(&pos, draft - 1, timer, -beta, -alpha)?;
                cutoff.fetch_max(score, Ordering::Relaxed);
                Ok(Some((m, score)))
            })
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(_, s)| s))))?
            .expect("expected at least one legal move");

        let transposition = if score >= beta {
            Transposition::lower(score, draft, best)
        } else if score <= alpha {
            Transposition::upper(score, draft, best)
        } else {
            Transposition::exact(score, draft, best)
        };

        self.tt.set(zobrist, transposition);

        Ok(score)
    }

    /// The [mtd(f)] algorithm.
    ///
    /// [mtd(f)]: https://en.wikipedia.org/wiki/MTD(f)
    fn mtdf(
        &self,
        pos: &Position,
        depth: i8,
        timer: Timer,
        mut score: i16,
    ) -> Result<i16, Timeout> {
        let mut alpha = -i16::MAX;
        let mut beta = i16::MAX;
        while alpha < beta {
            let target = score.max(alpha + 1);
            score = self.alpha_beta(pos, depth, timer, target - 1, target)?;
            if score < target {
                beta = score;
            } else {
                alpha = score;
            }
        }

        Ok(score)
    }
}

impl<E: Eval + Send + Sync> Search for Minimax<E> {
    fn search(&self, pos: &Position) -> Option<Move> {
        let timer = Timer::start(self.config.max_time);

        let zobrist = pos.zobrist();
        let (mut best, mut score, depth) = match self.tt.get(zobrist) {
            Some(t) => (Some(t.best()), t.score(), t.draft()),
            _ => (None, self.engine.eval(pos), 0),
        };

        for d in depth..self.config.max_depth.min(i8::MAX as u8) as i8 {
            match self.mtdf(pos, d + 1, timer, score) {
                Ok(s) => (best, score) = (self.tt.get(zobrist).map(|t| t.best()), s),
                Err(_) => break,
            }
        }

        best
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
    fn alpha_beta_panics_if_alpha_not_smaller_than_beta(
        c: MinimaxConfig,
        pos: Position,
        a: i16,
        b: i16,
    ) {
        let t = Timer::start(c.max_time);
        let d = c.max_depth.try_into()?;
        Minimax::new(MockEval::new()).alpha_beta(&pos, d, t, a.max(b), a.min(b))?;
    }

    #[proptest]
    fn alpha_beta_evaluates_position_if_depth_is_zero(pos: Position, s: i16) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let t = Timer::default();
        let strategy = Minimax::new(engine);
        assert_eq!(strategy.alpha_beta(&pos, 0, t, -i16::MAX, i16::MAX), Ok(s));
    }

    #[proptest]
    fn alpha_beta_aborts_if_time_is_up(c: MinimaxConfig, pos: Position) {
        let t = Timer {
            deadline: Some(Instant::now() - Duration::from_nanos(1)),
        };

        let d = c.max_depth.try_into()?;
        let strategy = Minimax::new(MockEval::new());

        assert_eq!(
            strategy.alpha_beta(&pos, d, t, -i16::MAX, i16::MAX),
            Err(Timeout)
        );
    }

    #[proptest]
    fn alpha_beta_returns_best_score(c: MinimaxConfig, pos: Position) {
        let t = Timer::default();
        let d = c.max_depth.try_into()?;
        let strategy = Minimax::with_config(Engine::default(), c);

        assert_eq!(
            strategy.alpha_beta(&pos, d, t, -i16::MAX, i16::MAX),
            Ok(minimax(&Engine::default(), &pos, d)),
        );
    }

    #[proptest]
    fn alpha_beta_does_not_depend_on_table_size(
        #[strategy(0usize..65536)] a: usize,
        #[strategy(0usize..65536)] b: usize,
        c: MinimaxConfig,
        pos: Position,
    ) {
        let t = Timer::default();
        let d = c.max_depth.try_into()?;

        let a = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: a, ..c });
        let b = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: b, ..c });

        assert_eq!(
            a.alpha_beta(&pos, d, t, -i16::MAX, i16::MAX),
            b.alpha_beta(&pos, d, t, -i16::MAX, i16::MAX)
        );
    }

    #[proptest]
    fn mtdf_returns_best_score(c: MinimaxConfig, pos: Position) {
        let t = Timer::default();
        let d = c.max_depth.try_into()?;
        let strategy = Minimax::with_config(Engine::default(), c);

        assert_eq!(
            strategy.mtdf(&pos, d, t, 0),
            Ok(minimax(&Engine::default(), &pos, d)),
        );
    }

    #[proptest]
    fn mtdf_aborts_if_time_is_up(c: MinimaxConfig, pos: Position, s: i16) {
        let t = Timer {
            deadline: Some(Instant::now() - Duration::from_nanos(1)),
        };

        let d = c.max_depth.try_into()?;
        let strategy = Minimax::with_config(Engine::default(), c);

        assert_eq!(strategy.mtdf(&pos, d, t, s), Err(Timeout));
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(c: MinimaxConfig, pos: Position, s: i16) {
        let t = Timer::default();
        let d = c.max_depth.try_into()?;

        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        assert_eq!(a.mtdf(&pos, d, t, s), b.mtdf(&pos, d, t, 0));
    }

    #[proptest]
    fn mtdf_is_equivalent_to_alpha_beta(c: MinimaxConfig, pos: Position) {
        let t = Timer::default();
        let d = c.max_depth.try_into()?;

        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        assert_eq!(
            a.mtdf(&pos, d, t, 0),
            b.alpha_beta(&pos, d, t, -i16::MAX, i16::MAX),
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

    #[proptest]
    fn search_can_be_limited_by_time(pos: Position, us: u16) {
        let c = MinimaxConfig {
            max_depth: u8::MAX,
            max_time: Duration::from_micros(us.into()),
            ..MinimaxConfig::default()
        };

        // should not hang
        Minimax::with_config(Engine::default(), c).search(&pos);
    }
}
