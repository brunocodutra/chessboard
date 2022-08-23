use crate::{Eval, MoveKind, Position, Pv, Transposition, TranspositionTable};
use crate::{Search, SearchLimits, SearchMetrics, SearchMetricsCounters};
use derive_more::{Display, Error, From};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{cmp::max_by_key, str::FromStr, time::Duration};
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
        }
    }
}

/// The reason why parsing [`MinimaxConfig`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse minimax configuration")]
pub struct ParseMinimaxConfigError(ron::de::SpannedError);

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
            tt: TranspositionTable::new(config.table_size),
        }
    }

    /// A null-window implementation of the [alpha-beta pruning] algorithm.
    ///
    /// [alpha-beta pruning]: https://en.wikipedia.org/wiki/Alpha%E2%80%93beta_pruning
    fn nw(
        &self,
        pos: &Position,
        guess: i16,
        draft: i8,
        time: Duration,
        counters: &SearchMetricsCounters,
    ) -> Result<i16, Timeout> {
        assert!(guess > i16::MIN, "{} > {}", guess, i16::MIN);

        if counters.time() >= time {
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
                if !t.bounds().contains(&guess) {
                    counters.tt_cut();
                    return Ok(t.score());
                }
            }
        }

        let quiesce = draft <= 0 && !pos.is_check();

        if draft <= Self::MIN_DRAFT {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        } else if quiesce {
            let stand_pat = self.engine.eval(pos).max(-i16::MAX);
            if stand_pat > guess {
                counters.sp_cut();
                #[cfg(not(test))]
                // The stand pat heuristic is not exact.
                return Ok(stand_pat);
            }
        } else if let Some(m) = transposition.map(|t| t.best()) {
            let mut pos = pos.clone();
            if pos.make(m).is_ok() {
                let score = -self.nw(&pos, -guess, draft - 1, time, counters)?;
                if score > guess {
                    counters.pv_cut();
                    self.tt.set(zobrist, Transposition::lower(score, draft, m));
                    return Ok(score);
                }
            }
        }

        let mut moves: Vec<_> = if quiesce {
            pos.moves(MoveKind::CAPTURE).collect()
        } else {
            pos.moves(MoveKind::ANY).collect()
        };

        if moves.is_empty() {
            return Ok(self.engine.eval(pos).max(-i16::MAX));
        }

        moves.sort_by_cached_key(|(_, _, pos)| self.engine.eval(pos));

        let cutoff = AtomicBool::new(false);

        let (best, score) = moves
            .into_par_iter()
            .map(|(m, _, pos)| {
                if cutoff.load(Ordering::Relaxed) {
                    Ok(None)
                } else {
                    let score = -self.nw(&pos, -guess, draft - 1, time, counters)?;
                    cutoff.fetch_or(score > guess, Ordering::Relaxed);
                    Ok(Some((m, score)))
                }
            })
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(_, s)| s))))?
            .expect("expected at least one legal move");

        self.tt.set(
            zobrist,
            if score > guess {
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
        mut score: i16,
        draft: i8,
        time: Duration,
        counters: &SearchMetricsCounters,
    ) -> Result<i16, Timeout> {
        let mut alpha = i16::MIN;
        let mut beta = i16::MAX;
        while alpha < beta {
            counters.test();
            let guess = score.max(alpha + 1);
            score = self.nw(pos, guess, draft, time, counters)?;
            if score < guess {
                beta = score;
            } else {
                alpha = score;
            }
        }

        Ok(score)
    }
}

impl<E: Eval + Send + Sync> Search for Minimax<E> {
    fn search<const N: usize>(&mut self, pos: &Position, limits: SearchLimits) -> Pv<N> {
        let mut pv: Pv<N> = self.tt.iter(pos).collect();

        let (mut score, start) = Option::zip(pv.score(), pv.depth()).unwrap_or_else(|| {
            self.tt.unset(pos.zobrist());
            (self.engine.eval(pos), 0)
        });

        let mut metrics = SearchMetrics::default();
        let mut counters = SearchMetricsCounters::default();
        for depth in start..=limits.depth().min(Self::MAX_DRAFT as u8) {
            let result = self.mtdf(pos, score, depth as i8, limits.time(), &counters);

            (score, pv) = match result {
                Ok(s) => (s, self.tt.iter(pos).collect()),
                Err(_) => break,
            };

            metrics = counters.snapshot() - metrics;
            debug!(depth, score, %pv, %metrics);
        }

        pv
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
    use tokio::{runtime, select, time::sleep};

    fn negamax<E: Eval + Send + Sync>(engine: &E, pos: &Position, draft: i8) -> i16 {
        pos.moves(MoveKind::ANY)
            .into_iter()
            .filter(|_| draft > Minimax::<E>::MIN_DRAFT)
            .filter(|(_, mk, _)| draft > 0 || pos.is_check() || mk.intersects(MoveKind::CAPTURE))
            .map(|(_, _, pos)| -negamax(engine, &pos, draft - 1))
            .max()
            .unwrap_or_else(|| engine.eval(pos).max(-i16::MAX))
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
        let mm = Minimax::with_config(MockEval::new(), c);
        prop_assume!(mm.tt.capacity() > 1);
        assert!(mm.tt.size() <= c.table_size);
    }

    #[proptest]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn nw_panics_if_beta_is_too_small(pos: Position, d: i8) {
        let mm = Minimax::new(MockEval::new());
        let counters = SearchMetricsCounters::default();
        mm.nw(&pos, i16::MIN, d, Duration::MAX, &counters)?;
    }

    #[proptest]
    fn nw_evaluates_position_if_draft_is_lower_than_minimum(
        pos: Position,
        #[strategy(-i16::MAX..)] g: i16,
        #[strategy(i8::MIN..=Minimax::<MockEval>::MIN_DRAFT)] d: i8,
        s: i16,
    ) {
        let mut engine = MockEval::new();
        engine
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let mm = Minimax::new(engine);
        let counters = SearchMetricsCounters::default();
        assert_eq!(mm.nw(&pos, g, d, Duration::MAX, &counters), Ok(s));
    }

    #[proptest]
    fn nw_aborts_if_time_is_up(
        mm: Minimax<Engine>,
        pos: Position,
        #[strategy(-i16::MAX..)] g: i16,
        d: i8,
    ) {
        let counters = SearchMetricsCounters::default();
        assert_eq!(mm.nw(&pos, g, d, Duration::ZERO, &counters), Err(Timeout));
    }

    #[proptest]
    fn mtdf_finds_best_score(
        mm: Minimax<Engine>,
        pos: Position,
        g: i16,
        #[strategy(Minimax::<Engine>::MIN_DRAFT..=Minimax::<Engine>::MAX_DRAFT)] d: i8,
    ) {
        assert_eq!(
            mm.mtdf(&pos, g, d, Duration::MAX, &SearchMetricsCounters::default()),
            Ok(negamax(&mm.engine, &pos, d))
        );
    }

    #[proptest]
    fn mtdf_aborts_if_time_is_up(
        mm: Minimax<Engine>,
        pos: Position,
        g: i16,
        #[strategy(Minimax::<Engine>::MIN_DRAFT..=Minimax::<Engine>::MAX_DRAFT)] d: i8,
    ) {
        let counters = SearchMetricsCounters::default();
        assert_eq!(mm.mtdf(&pos, g, d, Duration::ZERO, &counters), Err(Timeout));
    }

    #[proptest]
    fn mtdf_does_not_depend_on_initial_guess(
        e: Engine,
        c: MinimaxConfig,
        pos: Position,
        g: i16,
        #[strategy(Minimax::<Engine>::MIN_DRAFT..=Minimax::<Engine>::MAX_DRAFT)] d: i8,
    ) {
        let a = Minimax::with_config(e.clone(), c);
        let b = Minimax::with_config(e, c);

        assert_eq!(
            a.mtdf(&pos, g, d, Duration::MAX, &SearchMetricsCounters::default()),
            b.mtdf(&pos, 0, d, Duration::MAX, &SearchMetricsCounters::default())
        );
    }

    #[proptest]
    fn mtdf_does_not_depend_on_table_size(
        e: Engine,
        a: MinimaxConfig,
        b: MinimaxConfig,
        pos: Position,
        g: i16,
        #[strategy(Minimax::<Engine>::MIN_DRAFT..=Minimax::<Engine>::MAX_DRAFT)] d: i8,
    ) {
        let a = Minimax::with_config(e.clone(), a);
        let b = Minimax::with_config(e, b);

        assert_eq!(
            a.mtdf(&pos, g, d, Duration::MAX, &SearchMetricsCounters::default()),
            b.mtdf(&pos, g, d, Duration::MAX, &SearchMetricsCounters::default())
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(
        mut mm: Minimax<Engine>,
        pos: Position,
        l: SearchLimits,
    ) {
        assert_eq!(mm.search::<256>(&pos, l), mm.tt.iter(&pos).collect());
    }

    #[proptest]
    fn search_avoids_tt_collisions(
        mut mm: Minimax<Engine>,
        pos: Position,
        l: SearchLimits,
        t: Transposition,
    ) {
        mm.tt.set(pos.zobrist(), t);
        assert_eq!(mm.search::<1>(&pos, l), mm.tt.iter(&pos).collect());
    }

    #[proptest]
    fn search_is_stable(mut mm: Minimax<Engine>, pos: Position, l: SearchLimits) {
        assert_eq!(mm.search::<0>(&pos, l), mm.search::<0>(&pos, l));
    }

    #[proptest]
    fn search_can_be_limited_by_time(mut mm: Minimax<Engine>, pos: Position, us: u8) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let l = SearchLimits::Time(Duration::from_micros(us.into()));

        rt.block_on(async {
            select! {
                _ = async { mm.search::<1>(&pos, l) } => {}
                _ = sleep(Duration::from_millis(1)) => {
                    panic!()
                }
            }
        });
    }
}
