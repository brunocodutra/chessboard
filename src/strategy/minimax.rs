use crate::{Eval, Position, Pv, SearchMetrics, Transposition, TranspositionTable};
use crate::{Search, SearchLimits, SearchMetricsCounters};
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{cmp::max_by_key, str::FromStr};
use tracing::debug;

#[cfg(test)]
use proptest::prelude::*;

#[derive(Debug, Display, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display(fmt = "time is up!")]
pub struct Timeout;

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
    #[cfg_attr(test, strategy(0usize..=1024))]
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
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Minimax<E: Eval + Send + Sync> {
    engine: E,
    #[cfg_attr(test, any((Some(0), Some(Self::MAX_DRAFT as u8))))]
    limits: SearchLimits,
    #[cfg_attr(test, strategy(Just(SearchMetrics::default())))]
    metrics: SearchMetrics,
    #[cfg_attr(test, strategy(any::<MinimaxConfig>()
        .prop_map(|c| TranspositionTable::new(c.table_size)))
    )]
    tt: TranspositionTable,
}

impl<E: Eval + Send + Sync + Default> Default for Minimax<E> {
    fn default() -> Self {
        Self::new(E::default())
    }
}

impl<E: Eval + Send + Sync> Drop for Minimax<E> {
    fn drop(&mut self) {
        debug!(metrics = %self.metrics)
    }
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
            metrics: SearchMetrics::default(),
            tt: TranspositionTable::new(config.table_size),
        }
    }

    /// A null-window implementation of the [alpha-beta pruning] algorithm.
    ///
    /// [alpha-beta pruning]: https://en.wikipedia.org/wiki/Alpha%E2%80%93beta_pruning
    fn nw(
        &self,
        pos: &Position,
        draft: i8,
        beta: i16,
        counters: &SearchMetricsCounters,
    ) -> Result<i16, Timeout> {
        assert!(beta > -i16::MAX, "{} > {}", beta, -i16::MAX);

        if counters.time() >= self.limits.time {
            return Err(Timeout);
        }

        counters.node();

        let zobrist = pos.zobrist();
        let transposition = self.tt.get(zobrist);

        if transposition.is_some() {
            counters.tt_hit();
        }

        match transposition.filter(|t| t.draft() >= draft) {
            None => (),
            #[cfg(test)] // Probing larger draft is not exact.
            Some(t) if t.draft() != draft => (),
            Some(t) => {
                let (lower, upper) = t.bounds().into_inner();
                if beta <= lower || beta > upper {
                    counters.tt_cut();
                    return Ok(t.score());
                }
            }
        }

        if draft <= Self::MIN_DRAFT {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        } else if draft <= 0 {
            let stand_pat = self.engine.eval(pos).max(-i16::MAX);
            if stand_pat >= beta {
                counters.sp_cut();
                #[cfg(not(test))]
                // The stand pat heuristic is not exact.
                return Ok(stand_pat);
            }
        } else if let Some(m) = transposition.map(|t| t.best()) {
            let mut pos = pos.clone();
            if pos.make(m).is_ok() {
                let score = -self.nw(&pos, draft - 1, -beta + 1, counters)?;
                if score >= beta {
                    counters.pv_cut();
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
                    let score = -self.nw(&pos, draft - 1, -beta + 1, counters)?;
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
        mut score: i16,
        counters: &SearchMetricsCounters,
    ) -> Result<i16, Timeout> {
        let mut alpha = -i16::MAX;
        let mut beta = i16::MAX;
        while alpha < beta {
            let target = score.max(alpha + 1);
            score = self.nw(pos, depth, target, counters)?;
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
    fn limits(&self) -> SearchLimits {
        self.limits
    }

    fn search(&mut self, pos: &Position) -> Pv {
        let (mut score, start) = match self.tt.pv(pos.clone()).next() {
            Some(t) if t.draft() >= 0 => (t.score(), t.draft() + 1),
            _ => {
                self.tt.unset(pos.zobrist());
                (self.engine.eval(pos), 0)
            }
        };

        let mut metrics = SearchMetrics::default();
        let mut counters = SearchMetricsCounters::default();
        for d in start..=self.limits.depth.min(Self::MAX_DRAFT as u8) as i8 {
            let result = self.mtdf(pos, d, score, &counters);
            metrics = counters.snapshot() - metrics;

            debug!(depth = d, %metrics);

            match result {
                Ok(s) => score = s,
                Err(_) => break,
            }
        }

        self.metrics += counters.snapshot();

        self.tt.pv(pos.clone())
    }

    fn clear(&mut self) {
        self.tt.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Engine, MockEval};
    use mockall::predicate::*;
    use proptest::prop_assume;
    use std::time::Duration;
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
            Minimax::new(MockEval::new()).limits(),
            SearchLimits::default()
        );
    }

    #[proptest]
    fn table_size_is_an_upper_limit(c: MinimaxConfig) {
        let mm = Minimax::with_config(MockEval::new(), c);
        prop_assume!(mm.tt.capacity() > 1);
        assert!(mm.tt.size() <= c.table_size);
    }

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn nw_panics_if_beta_is_too_small(pos: Position, d: i8, #[strategy(..=-i16::MAX)] b: i16) {
        Minimax::new(MockEval::new()).nw(&pos, d, b, &SearchMetricsCounters::default())?;
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

        let mm = Minimax::new(engine);
        assert_eq!(mm.nw(&pos, d, b, &SearchMetricsCounters::default()), Ok(s));
    }

    #[proptest]
    fn nw_aborts_if_time_is_up(
        c: MinimaxConfig,
        pos: Position,
        #[strategy(-i16::MAX + 1..)] b: i16,
    ) {
        let d = c.search.depth.try_into()?;

        let mm = Minimax::with_config(
            MockEval::new(),
            MinimaxConfig {
                search: SearchLimits {
                    time: Duration::ZERO,
                    ..c.search
                },
                ..c
            },
        );

        assert_eq!(
            mm.nw(&pos, d, b, &SearchMetricsCounters::default()),
            Err(Timeout)
        );
    }

    #[proptest]
    fn mtdf_finds_best_score(mm: Minimax<Engine>, pos: Position, s: i16) {
        let d = mm.limits.depth.try_into()?;
        assert_eq!(
            mm.mtdf(&pos, d, s, &SearchMetricsCounters::default()),
            Ok(negamax(&mm.engine, &pos, d))
        );
    }

    #[proptest]
    fn mtdf_aborts_if_time_is_up(c: MinimaxConfig, pos: Position, s: i16) {
        let d = c.search.depth.try_into()?;

        let mm = Minimax::with_config(
            MockEval::new(),
            MinimaxConfig {
                search: SearchLimits {
                    time: Duration::ZERO,
                    ..c.search
                },
                ..c
            },
        );

        assert_eq!(
            mm.mtdf(&pos, d, s, &SearchMetricsCounters::default()),
            Err(Timeout)
        );
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(c: MinimaxConfig, pos: Position, s: i16) {
        let d = c.search.depth.try_into()?;

        let a = Minimax::with_config(Engine::default(), c);
        let b = Minimax::with_config(Engine::default(), c);

        assert_eq!(
            a.mtdf(&pos, d, s, &SearchMetricsCounters::default()),
            b.mtdf(&pos, d, 0, &SearchMetricsCounters::default())
        );
    }

    #[proptest]
    fn mtdf_does_not_depend_on_table_size(
        #[strategy(0usize..=1024)] x: usize,
        #[strategy(0usize..=1024)] y: usize,
        c: MinimaxConfig,
        pos: Position,
        s: i16,
    ) {
        let d = c.search.depth.try_into()?;

        let x = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: x, ..c });
        let y = Minimax::with_config(Engine::default(), MinimaxConfig { table_size: y, ..c });

        assert_eq!(
            x.mtdf(&pos, d, s, &SearchMetricsCounters::default()),
            y.mtdf(&pos, d, s, &SearchMetricsCounters::default())
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(mut mm: Minimax<Engine>, pos: Position) {
        assert_eq!(mm.search(&pos).next(), mm.tt.get(pos.zobrist()));
    }

    #[proptest]
    fn search_avoids_tt_collisions(mut mm: Minimax<Engine>, pos: Position, t: Transposition) {
        mm.tt.set(pos.zobrist(), t);
        assert_eq!(mm.search(&pos).next(), mm.tt.get(pos.zobrist()));
    }

    #[proptest]
    fn search_is_stable(mut mm: Minimax<Engine>, pos: Position) {
        assert_eq!(mm.search(&pos).next(), mm.search(&pos).next());
    }

    #[proptest]
    fn search_can_be_limited_by_time(e: Engine, mut c: MinimaxConfig, pos: Position, us: u16) {
        c.search = SearchLimits {
            time: Duration::from_micros(us.into()),
            depth: u8::MAX,
        };

        // should not hang
        Minimax::with_config(e, c).search(&pos);
    }
}
