use crate::{Eval, Move, Position, Search, SearchLimits, Transposition, TranspositionTable};
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{cmp::max_by_key, fmt::Debug, str::FromStr};

#[derive(Debug, Display, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display(fmt = "time is up!")]
pub struct Timeout;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
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
    /// Search limits.
    #[cfg_attr(test, any((Some(0), Some(Minimax::<crate::MockEval>::MAX_DRAFT as u8))))]
    pub search: SearchLimits,

    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[cfg_attr(test, strategy(0usize..=65536))]
    pub table_size: usize,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
        Self {
            table_size: 1 << 24,
            search: SearchLimits::default(),
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
    limits: SearchLimits,
    tt: TranspositionTable,
}

impl<E: Eval + Send + Sync> Minimax<E> {
    #[cfg(test)]
    #[cfg(tarpaulin)]
    const MIN_DRAFT: i8 = -1;
    #[cfg(test)]
    #[cfg(tarpaulin)]
    const MAX_DRAFT: i8 = 1;

    #[cfg(test)]
    #[cfg(not(tarpaulin))]
    const MIN_DRAFT: i8 = -2;
    #[cfg(test)]
    #[cfg(not(tarpaulin))]
    const MAX_DRAFT: i8 = 2;

    #[cfg(not(test))]
    const MIN_DRAFT: i8 = Transposition::MIN_DRAFT;
    #[cfg(not(test))]
    const MAX_DRAFT: i8 = Transposition::MAX_DRAFT;

    /// Constructs [`Minimax`] with the default [`MinimaxConfig`].
    pub fn new(engine: E) -> Self {
        Self::with_config(engine, MinimaxConfig::default())
    }

    /// Constructs [`Minimax`] with some [`MinimaxConfig`].
    pub fn with_config(engine: E, config: MinimaxConfig) -> Self {
        Minimax {
            engine,
            limits: config.search,
            tt: TranspositionTable::new(config.table_size),
        }
    }

    /// A null-window implementation of the [alpha-beta pruning] algorithm.
    ///
    /// [alpha-beta pruning]: https://en.wikipedia.org/wiki/Alpha%E2%80%93beta_pruning
    fn nw(&self, pos: &Position, draft: i8, timer: Timer, beta: i16) -> Result<i16, Timeout> {
        assert!(beta > -i16::MAX, "{} > {}", beta, -i16::MAX);

        timer.elapsed()?;

        let zobrist = pos.zobrist();
        let transposition = self.tt.get(zobrist);

        match transposition.filter(|t| t.draft() >= draft) {
            None => (),
            #[cfg(test)] // Probing larger draft is not exact.
            Some(t) if t.draft() != draft => (),
            Some(t) => {
                let (lower, upper) = t.bounds().into_inner();
                if beta <= lower || beta > upper {
                    return Ok(t.score());
                }
            }
        }

        if draft <= Self::MIN_DRAFT {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        } else if draft <= 0 {
            let stand_pat = self.engine.eval(pos).max(-i16::MAX);
            if stand_pat >= beta {
                #[cfg(not(test))]
                // The stand pat heuristic is not exact.
                return Ok(stand_pat);
            }
        } else if let Some(m) = transposition.map(|t| t.best()) {
            let mut pos = pos.clone();
            if pos.play(m).is_ok() {
                let score = -self.nw(&pos, draft - 1, timer, -beta + 1)?;
                if score >= beta {
                    self.tt.set(zobrist, Transposition::lower(score, draft, m));
                    return Ok(score);
                }
            }
        }

        let mut moves: Vec<_> = if draft <= 0 {
            pos.captures().collect()
        } else {
            pos.moves().collect()
        };

        if moves.is_empty() {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        }

        moves.sort_by_cached_key(|(_, pos)| self.engine.eval(pos));

        let cutoff = AtomicBool::new(false);

        let (best, score) = moves
            .into_par_iter()
            .map(|(m, pos)| {
                if cutoff.load(Ordering::Relaxed) {
                    Ok(None)
                } else {
                    let score = -self.nw(&pos, draft - 1, timer, -beta + 1)?;
                    cutoff.fetch_or(score >= beta, Ordering::Relaxed);
                    Ok(Some((m, score)))
                }
            })
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(_, s)| s))))?
            .expect("expected at least one legal move");

        self.tt.set(
            zobrist,
            if score >= beta {
                Transposition::lower(score, draft, best)
            } else {
                Transposition::upper(score, draft, best)
            },
        );

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
            score = self.nw(pos, depth, timer, target)?;
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
        let timer = Timer::start(self.limits.time);

        let zobrist = pos.zobrist();
        let (mut score, mut best, depth) = match self.tt.get(zobrist) {
            Some(t) if t.draft() >= 0 => (t.score(), Some(t.best()), t.draft() + 1),
            _ => (self.engine.eval(pos), None, 0),
        };

        for d in depth..=self.limits.depth.min(Self::MAX_DRAFT as u8) as i8 {
            (score, best) = match self.mtdf(pos, d, timer, score) {
                Ok(s) => (s, self.tt.get(zobrist).map(|t| t.best())),
                Err(_) => break,
            };
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

    fn quiesce<E: Eval + Send + Sync>(engine: &E, pos: &Position, draft: i8) -> i16 {
        if draft <= Minimax::<E>::MIN_DRAFT {
            engine.eval(pos).max(-i16::MAX)
        } else {
            pos.captures()
                .map(|(_, pos)| -quiesce(engine, &pos, draft - 1))
                .max()
                .unwrap_or_else(|| engine.eval(pos).max(-i16::MAX))
        }
    }

    fn negamax<E: Eval + Send + Sync>(engine: &E, pos: &Position, draft: i8) -> i16 {
        if draft <= 0 {
            quiesce(engine, pos, draft)
        } else {
            pos.moves()
                .map(|(_, pos)| -negamax(engine, &pos, draft - 1))
                .max()
                .unwrap_or_else(|| engine.eval(pos).max(-i16::MAX))
        }
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
    fn new_applies_default_search_limits() {
        assert_eq!(
            Minimax::new(MockEval::new()).limits,
            SearchLimits::default()
        );
    }

    #[proptest]
    fn table_size_is_an_upper_limit(c: MinimaxConfig) {
        let strategy = Minimax::with_config(MockEval::new(), c);
        assert!(strategy.tt.size() <= c.table_size);
    }

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn nw_panics_if_beta_is_too_small(
        pos: Position,
        d: i8,
        t: Timer,
        #[strategy(..=-i16::MAX)] b: i16,
    ) {
        Minimax::new(MockEval::new()).nw(&pos, d, t, b)?;
    }

    #[proptest]
    fn nw_evaluates_position_if_draft_is_lower_than_minimum(
        pos: Position,
        #[strategy(i8::MIN..=Minimax::<MockEval>::MIN_DRAFT)] d: i8,
        s: i16,
        #[strategy(-i16::MAX + 1..)] b: i16,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let t = Timer::default();
        let strategy = Minimax::new(engine);
        assert_eq!(strategy.nw(&pos, d, t, b), Ok(s));
    }

    #[proptest]
    fn nw_aborts_if_time_is_up(pos: Position, d: i8, #[strategy(-i16::MAX + 1..)] b: i16) {
        let t = Timer {
            deadline: Some(Instant::now() - Duration::from_nanos(1)),
        };

        let strategy = Minimax::new(MockEval::new());
        assert_eq!(strategy.nw(&pos, d, t, b), Err(Timeout));
    }

    #[proptest]
    fn mtdf_finds_best_score(c: MinimaxConfig, pos: Position, s: i16) {
        let t = Timer::default();
        let d = c.search.depth.try_into()?;
        let strategy = Minimax::with_config(Engine::default(), c);

        assert_eq!(
            strategy.mtdf(&pos, d, t, s),
            Ok(negamax(&strategy.engine, &pos, d)),
        );
    }

    #[proptest]
    fn mtdf_aborts_if_time_is_up(c: MinimaxConfig, pos: Position, s: i16) {
        let t = Timer {
            deadline: Some(Instant::now() - Duration::from_nanos(1)),
        };

        let d = c.search.depth.try_into()?;
        let strategy = Minimax::with_config(Engine::default(), c);

        assert_eq!(strategy.mtdf(&pos, d, t, s), Err(Timeout));
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(c: MinimaxConfig, pos: Position, s: i16) {
        let t = Timer::default();
        let d = c.search.depth.try_into()?;

        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        assert_eq!(a.mtdf(&pos, d, t, s), b.mtdf(&pos, d, t, 0));
    }

    #[proptest]
    fn mtdf_does_not_depend_on_table_size(
        #[strategy(0usize..65536)] x: usize,
        #[strategy(0usize..65536)] y: usize,
        c: MinimaxConfig,
        pos: Position,
        s: i16,
    ) {
        let t = Timer::default();
        let d = c.search.depth.try_into()?;

        let x = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: x, ..c });
        let y = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: y, ..c });

        assert_eq!(x.mtdf(&pos, d, t, s), y.mtdf(&pos, d, t, s));
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
    fn search_can_be_limited_by_time(mut c: MinimaxConfig, pos: Position, us: u16) {
        c.search = SearchLimits {
            time: Duration::from_micros(us.into()),
            depth: u8::MAX,
        };

        // should not hang
        Minimax::with_config(Engine::default(), c).search(&pos);
    }
}
