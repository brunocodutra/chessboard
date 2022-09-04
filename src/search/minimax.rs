use crate::chess::{Move, MoveKind, Piece, Position, Role};
use crate::search::{Limits, Metrics, MetricsCounters};
use crate::{Eval, Pv, Search, Transposition, TranspositionTable};
use derive_more::{Deref, Display, Error, From, Neg};
use rayon::{iter::once, prelude::*};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cmp::max_by_key, ops::Range, str::FromStr, time::Duration};
use tracing::debug;

#[cfg(test)]
use proptest::prelude::*;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref, Neg)]
struct Score(#[deref] i16, i8);

impl Score {
    const MIN: Self = Score(i16::MIN, i8::MIN);
    const MAX: Self = Score(i16::MAX, i8::MAX);
}

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
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
    pub hash: usize,
}

impl Default for MinimaxConfig {
    fn default() -> Self {
        Self { hash: 1 << 25 }
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
/// [minimax]: https://www.chessprogramming.org/Minimax
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Minimax<E: Eval> {
    evaluation: E,
    #[cfg_attr(test, strategy(any::<MinimaxConfig>()
        .prop_map(|c| TranspositionTable::new(c.hash)))
    )]
    tt: TranspositionTable,
}

impl<E: Eval + Default> Default for Minimax<E> {
    fn default() -> Self {
        Self::new(E::default())
    }
}

impl<E: Eval> Minimax<E> {
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
    pub fn new(evaluation: E) -> Self {
        Self::with_config(evaluation, MinimaxConfig::default())
    }

    /// Constructs [`Minimax`] with some [`MinimaxConfig`].
    pub fn with_config(evaluation: E, config: MinimaxConfig) -> Self {
        Minimax {
            evaluation,
            tt: TranspositionTable::new(config.hash),
        }
    }

    fn eval(&self, pos: &Position, draft: i8) -> Score {
        Score(self.evaluation.eval(pos).max(-i16::MAX), draft)
    }
}

impl<E: Eval + Send + Sync> Minimax<E> {
    /// An implementation of [aspiration windows].
    ///
    /// [aspiration windows]: https://www.chessprogramming.org/Aspiration_Windows
    fn aw(
        &self,
        pos: &Position,
        guess: i16,
        draft: i8,
        time: Duration,
        metrics: &MetricsCounters,
    ) -> Result<Score, Timeout> {
        const W: i16 = 64;

        let upper = guess.saturating_add(W / 2).max(W - i16::MAX);
        let lower = guess.saturating_sub(W / 2).min(i16::MAX - W).max(-i16::MAX);
        let score = self.pvs(None, pos, lower..upper, draft, time, metrics)?;

        if *score >= upper {
            self.pvs(None, pos, lower..i16::MAX, draft, time, metrics)
        } else if *score <= lower {
            self.pvs(None, pos, -i16::MAX..upper, draft, time, metrics)
        } else {
            Ok(score)
        }
    }

    /// A [zero-window] wrapper for [`Self::pvs`].
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw(
        &self,
        prev: Option<Move>,
        pos: &Position,
        bound: i16,
        draft: i8,
        time: Duration,
        metrics: &MetricsCounters,
    ) -> Result<Score, Timeout> {
        assert!(bound < i16::MAX, "{} < {}", bound, i16::MAX);
        self.pvs(prev, pos, bound..bound + 1, draft, time, metrics)
    }

    /// An implementation of the [PVS] variation of [alpha-beta pruning] algorithm.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn pvs(
        &self,
        prev: Option<Move>,
        pos: &Position,
        bounds: Range<i16>,
        draft: i8,
        time: Duration,
        metrics: &MetricsCounters,
    ) -> Result<Score, Timeout> {
        assert!(!bounds.is_empty(), "{:?} ≠ ∅", bounds);
        assert!(!bounds.contains(&i16::MIN), "{:?} ∌ {}", bounds, i16::MIN);

        let (mut alpha, mut beta) = (bounds.start, bounds.end);

        if metrics.time() >= time {
            return Err(Timeout);
        }

        metrics.node();

        let zobrist = pos.zobrist();
        let transposition = self.tt.get(zobrist);

        if transposition.is_some() {
            metrics.tt_hit();
        }

        match transposition.filter(|t| t.draft() >= draft) {
            None => (),
            #[cfg(test)] // Probing larger draft is not exact.
            Some(t) if t.draft() != draft => (),
            Some(t) => {
                let (lower, upper) = t.bounds().into_inner();
                (alpha, beta) = (alpha.max(lower), beta.min(upper));
                if alpha >= beta {
                    metrics.tt_cut();
                    return Ok(Score(t.score(), draft));
                }
            }
        }

        let stand_pat = self.eval(pos, draft);
        let quiesce = draft <= 0 && !pos.is_check();

        if draft <= Self::MIN_DRAFT {
            return Ok(stand_pat);
        } else if quiesce {
            if cfg!(not(test)) {
                // The stand pat heuristic is not exact.
                alpha = alpha.max(*stand_pat)
            }

            if alpha >= beta {
                metrics.sp_cut();
                return Ok(stand_pat);
            }
        } else if draft > 0 && *stand_pat >= beta && prev.is_some()
                // Avoid common zugzwang positions in which the side to move only has pawns.
                && pos.by_color(pos.turn()).len() - 1 > pos.by_piece(Piece(pos.turn(), Role::Pawn)).len()
        {
            let mut pos = pos.clone();
            if pos.pass().is_ok() {
                let r = (2 + draft / 8).min(draft - 1);
                if *-self.nw(None, &pos, -beta, draft - r - 1, time, metrics)? >= beta {
                    metrics.nm_cut();
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Score(beta, draft));
                }
            }
        }

        let move_kinds = if quiesce {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
        };

        let mut moves: Vec<_> = pos.moves(move_kinds).collect();
        moves.sort_by_cached_key(|(m, _, p)| {
            if transposition.map(|t| t.best()) == Some(*m) {
                Score::MAX
            } else {
                -self.eval(p, draft - 1)
            }
        });

        let (score, pv) = match moves.pop() {
            None => return Ok(stand_pat),
            Some((m, _, pos)) => {
                let score = -self.pvs(Some(m), &pos, -beta..-alpha, draft - 1, time, metrics)?;

                if *score >= beta {
                    metrics.pv_cut();
                    self.tt.set(zobrist, Transposition::lower(*score, draft, m));
                    return Ok(score);
                }

                (score, m)
            }
        };

        let cutoff = AtomicI16::new(alpha.max(*score));

        let (score, best) = moves
            .into_par_iter()
            .with_max_len(1)
            .rev()
            .map(|(m, _, pos)| {
                let mut score = Score::MIN;
                let mut alpha = cutoff.load(Ordering::Relaxed);
                while *score < alpha && alpha < beta {
                    let target = alpha;
                    score = -self.nw(Some(m), &pos, -target - 1, draft - 1, time, metrics)?;
                    alpha = cutoff.fetch_max(*score, Ordering::Relaxed).max(*score);
                    if *score < target {
                        break;
                    }
                }

                if alpha <= *score && *score < beta {
                    score = -self.pvs(Some(m), &pos, -beta..-alpha, draft - 1, time, metrics)?;
                    cutoff.fetch_max(*score, Ordering::Relaxed);
                }

                Ok(Some((score, m)))
            })
            .chain(once(Ok(Some((score, pv)))))
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(s, _)| s))))?
            .expect("expected at least one legal move");

        self.tt.set(
            zobrist,
            if *score >= beta {
                Transposition::lower(*score, draft, best)
            } else if *score <= alpha {
                Transposition::upper(*score, draft, best)
            } else {
                Transposition::exact(*score, draft, best)
            },
        );

        Ok(score)
    }
}

impl<E: Eval + Send + Sync> Search for Minimax<E> {
    fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let mut pv: Pv<N> = self.tt.iter(pos).collect();

        let (mut score, start) = Option::zip(pv.score(), pv.depth()).unwrap_or_else(|| {
            self.tt.unset(pos.zobrist());
            (self.evaluation.eval(pos), 0)
        });

        let mut metrics = Metrics::default();
        let mut counters = MetricsCounters::default();
        for depth in start..=limits.depth().min(Self::MAX_DRAFT as u8) {
            (score, pv) = match self.aw(pos, score, depth as i8, limits.time(), &counters) {
                Ok(s) => (*s, self.tt.iter(pos).collect()),
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
    use crate::{eval::Dispatcher as Evaluator, MockEval};
    use mockall::predicate::*;
    use proptest::prop_assume;
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::{runtime, select, time::sleep};

    fn negamax<E: Eval>(evaluation: &E, pos: &Position, draft: i8) -> i16 {
        let score = evaluation.eval(pos).max(-i16::MAX);

        let move_kinds = if draft <= Minimax::<E>::MIN_DRAFT {
            return score;
        } else if draft <= 0 && !pos.is_check() {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
        };

        pos.moves(move_kinds)
            .map(|(_, _, pos)| -negamax(evaluation, &pos, draft - 1))
            .max()
            .unwrap_or(score)
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
        assert!(mm.tt.size() <= c.hash);
    }

    #[proptest]
    #[should_panic]
    fn nw_panics_if_bound_is_too_large(m: Option<Move>, pos: Position, d: i8) {
        let mm = Minimax::new(MockEval::new());
        let metrics = MetricsCounters::default();
        mm.nw(m, &pos, i16::MAX, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_too_small(m: Option<Move>, pos: Position, b: i16, d: i8) {
        let mm = Minimax::new(MockEval::new());
        let metrics = MetricsCounters::default();
        mm.pvs(m, &pos, i16::MIN..b, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_not_greater_than_beta(
        m: Option<Move>,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(..=#a)] b: i16,
        d: i8,
    ) {
        let mm = Minimax::new(MockEval::new());
        let metrics = MetricsCounters::default();
        mm.pvs(m, &pos, a..b, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    fn pvs_evaluates_position_if_draft_is_lower_than_minimum(
        m: Option<Move>,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(#a+1..)] b: i16,
        #[strategy(i8::MIN..=Minimax::<MockEval>::MIN_DRAFT)] d: i8,
        #[strategy(-i16::MAX..)] s: i16,
    ) {
        let mut evaluation = MockEval::new();
        evaluation
            .expect_eval()
            .once()
            .with(eq(pos.clone()))
            .return_const(s);

        let mm = Minimax::new(evaluation);
        let metrics = MetricsCounters::default();
        assert_eq!(
            mm.pvs(m, &pos, a..b, d, Duration::MAX, &metrics),
            Ok(Score(s, d))
        );
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        mm: Minimax<Evaluator>,
        m: Option<Move>,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(#a+1..)] b: i16,
        d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            mm.pvs(m, &pos, a..b, d, Duration::ZERO, &metrics),
            Err(Timeout)
        );
    }

    #[proptest]
    fn pvs_finds_best_score(
        mm: Minimax<Evaluator>,
        m: Option<Move>,
        pos: Position,
        #[strategy(Minimax::<Evaluator>::MIN_DRAFT..=Minimax::<Evaluator>::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            mm.pvs(m, &pos, -i16::MAX..i16::MAX, d, Duration::MAX, &metrics)
                .as_deref(),
            Ok(&negamax(&mm.evaluation, &pos, d))
        );
    }

    #[proptest]
    fn pvs_does_not_depend_on_table_size(
        e: Evaluator,
        x: MinimaxConfig,
        y: MinimaxConfig,
        m: Option<Move>,
        pos: Position,
        #[strategy(Minimax::<Evaluator>::MIN_DRAFT..=Minimax::<Evaluator>::MAX_DRAFT)] d: i8,
    ) {
        let x = Minimax::with_config(e.clone(), x);
        let y = Minimax::with_config(e, y);

        let xc = MetricsCounters::default();
        let yc = MetricsCounters::default();

        assert_eq!(
            x.pvs(m, &pos, -i16::MAX..i16::MAX, d, Duration::MAX, &xc)
                .as_deref(),
            y.pvs(m, &pos, -i16::MAX..i16::MAX, d, Duration::MAX, &yc)
                .as_deref()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(mut mm: Minimax<Evaluator>, pos: Position, l: Limits) {
        assert_eq!(mm.search::<256>(&pos, l), mm.tt.iter(&pos).collect());
    }

    #[proptest]
    fn search_avoids_tt_collisions(
        mut mm: Minimax<Evaluator>,
        pos: Position,
        l: Limits,
        t: Transposition,
    ) {
        mm.tt.set(pos.zobrist(), t);
        assert_eq!(mm.search::<1>(&pos, l), mm.tt.iter(&pos).collect());
    }

    #[proptest]
    fn search_is_stable(mut mm: Minimax<Evaluator>, pos: Position, l: Limits) {
        assert_eq!(mm.search::<0>(&pos, l), mm.search::<0>(&pos, l));
    }

    #[proptest]
    fn search_can_be_limited_by_time(mut mm: Minimax<Evaluator>, pos: Position, us: u8) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let l = Limits::Time(Duration::from_micros(us.into()));

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
