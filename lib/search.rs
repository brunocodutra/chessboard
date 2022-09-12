use crate::chess::{Move, MoveKind, Piece, Position, Role};
use crate::eval::{Eval, Evaluator};
use crate::transposition::{Table, Transposition};
use derive_more::{Deref, Display, Error, Neg};
use proptest::prelude::*;
use rayon::{iter::once, prelude::*};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cmp::max_by_key, ops::Range, time::Duration};
use test_strategy::Arbitrary;
use tracing::debug;

mod limits;
mod metrics;
mod options;
mod pv;

pub use limits::*;
pub use metrics::*;
pub use options::*;
pub use pv::*;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref, Neg)]
struct Score(#[deref] i16, i8);

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display(fmt = "time is up!")]
pub struct Timeout;

/// An implementation of [minimax].
///
/// [minimax]: https://www.chessprogramming.org/Searcher
#[derive(Debug, Arbitrary)]
pub struct Searcher {
    evaluator: Evaluator,
    #[strategy(any::<Options>().prop_map(|o| Table::new(o.hash)))]
    tt: Table,
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new(Evaluator::default())
    }
}

impl Searcher {
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

    /// Constructs [`Searcher`] with the default [`Options`].
    pub fn new(evaluator: Evaluator) -> Self {
        Self::with_options(evaluator, Options::default())
    }

    /// Constructs [`Searcher`] with the given [`Options`].
    pub fn with_options(evaluator: Evaluator, options: Options) -> Self {
        Searcher {
            evaluator,
            tt: Table::new(options.hash),
        }
    }

    fn eval(&self, pos: &Position, draft: i8) -> Score {
        Score(self.evaluator.eval(pos).max(-i16::MAX), draft)
    }

    fn moves(
        &self,
        pos: &Position,
        kind: MoveKind,
        transposition: Option<Transposition>,
    ) -> Vec<(Move, MoveKind, Position)> {
        let mut moves: Vec<_> = pos.moves(kind).collect();

        moves.sort_by_cached_key(|(m, _, p)| {
            if transposition.map(|t| t.best()) == Some(*m) {
                i16::MAX
            } else {
                let exchange = p.see(m.whither()).rev().fold(0i16, |v, (r, p)| {
                    let capture = self.evaluator.eval(&r);
                    let promotion = self.evaluator.eval(&p);
                    promotion.saturating_sub(v).saturating_add(capture).max(0)
                });

                let capture = pos[m.whither()]
                    .map(|p| self.evaluator.eval(&p.role()))
                    .unwrap_or(0);

                let promotion = self.evaluator.eval(&m.promotion());

                promotion.saturating_sub(exchange).saturating_add(capture)
            }
        });

        moves
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(&self, pos: &Position, draft: i8) -> Option<i8> {
        let turn = pos.turn();

        // Avoid common zugzwang positions in which the side to move only has pawns.
        if draft > 0 && pos.by_color(turn).len() > pos.by_piece(Piece(turn, Role::Pawn)).len() + 1 {
            let r = 2 + draft / 8;
            Some((draft - r - 1).max(0))
        } else {
            None
        }
    }

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
        } else if *stand_pat >= beta && prev.is_some() {
            if let Some(d) = self.nmp(pos, draft) {
                let mut next = pos.clone();
                if next.pass().is_ok() && *-self.nw(None, &next, -beta, d, time, metrics)? >= beta {
                    metrics.nm_cut();
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Score(beta, draft));
                }
            }
        }

        let kind = if quiesce {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
        };

        let mut moves = self.moves(pos, kind, transposition);

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
                let mut score = Score(i16::MIN, Self::MIN_DRAFT);
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

    /// Searches for the strongest [variation][`Pv`].
    pub fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let mut pv: Pv<N> = self.tt.iter(pos).collect();

        let (mut score, start) = Option::zip(pv.score(), pv.depth()).unwrap_or_else(|| {
            self.tt.unset(pos.zobrist());
            (self.evaluator.eval(pos), 0)
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

    /// Clear the transposition table.
    pub fn clear(&mut self) {
        self.tt.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prop_assume;
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::{runtime, time::timeout};

    fn negamax(evaluator: &Evaluator, pos: &Position, draft: i8) -> i16 {
        let score = evaluator.eval(pos).max(-i16::MAX);

        let kind = if draft <= Searcher::MIN_DRAFT {
            return score;
        } else if draft <= 0 && !pos.is_check() {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
        };

        pos.moves(kind)
            .map(|(_, _, pos)| -negamax(evaluator, &pos, draft - 1))
            .max()
            .unwrap_or(score)
    }

    #[proptest]
    fn table_size_is_an_upper_limit(o: Options) {
        let s = Searcher::with_options(Evaluator::default(), o);
        prop_assume!(s.tt.capacity() > 1);
        assert!(s.tt.size() <= o.hash);
    }

    #[proptest]
    #[should_panic]
    fn nw_panics_if_bound_is_too_large(s: Searcher, m: Option<Move>, pos: Position, d: i8) {
        let metrics = MetricsCounters::default();
        s.nw(m, &pos, i16::MAX, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_too_small(
        s: Searcher,
        m: Option<Move>,
        pos: Position,
        b: i16,
        d: i8,
    ) {
        let metrics = MetricsCounters::default();
        s.pvs(m, &pos, i16::MIN..b, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_not_greater_than_beta(
        s: Searcher,
        m: Option<Move>,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(..=#a)] b: i16,
        d: i8,
    ) {
        let metrics = MetricsCounters::default();
        s.pvs(m, &pos, a..b, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    fn pvs_evaluates_position_if_draft_is_lower_than_minimum(
        s: Searcher,
        m: Option<Move>,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(#a + 1..)] b: i16,
        #[strategy(i8::MIN..=Searcher::MIN_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.pvs(m, &pos, a..b, d, Duration::MAX, &metrics),
            Ok(s.eval(&pos, d))
        );
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        s: Searcher,
        m: Option<Move>,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(#a+1..)] b: i16,
        d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.pvs(m, &pos, a..b, d, Duration::ZERO, &metrics),
            Err(Timeout)
        );
    }

    #[proptest]
    fn pvs_finds_best_score(
        s: Searcher,
        m: Option<Move>,
        pos: Position,
        #[strategy(Searcher::MIN_DRAFT..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.pvs(m, &pos, -i16::MAX..i16::MAX, d, Duration::MAX, &metrics)
                .as_deref(),
            Ok(&negamax(&s.evaluator, &pos, d))
        );
    }

    #[proptest]
    fn pvs_does_not_depend_on_table_size(
        e: Evaluator,
        x: Options,
        y: Options,
        m: Option<Move>,
        pos: Position,
        #[strategy(Searcher::MIN_DRAFT..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let x = Searcher::with_options(e.clone(), x);
        let y = Searcher::with_options(e, y);

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
    fn aw_finds_best_score(
        s: Searcher,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] g: i16,
        #[strategy(Searcher::MIN_DRAFT..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.aw(&pos, g, d, Duration::MAX, &metrics).as_deref(),
            Ok(&negamax(&s.evaluator, &pos, d))
        );
    }

    #[proptest]
    fn aw_does_not_depend_on_initial_guess(
        s: Searcher,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] g: i16,
        #[strategy(-i16::MAX..i16::MAX)] h: i16,
        #[strategy(Searcher::MIN_DRAFT..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.aw(&pos, g, d, Duration::MAX, &metrics).as_deref(),
            s.aw(&pos, h, d, Duration::MAX, &metrics).as_deref()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(mut s: Searcher, pos: Position, l: Limits) {
        assert_eq!(s.search::<256>(&pos, l), s.tt.iter(&pos).collect());
    }

    #[proptest]
    fn search_avoids_tt_collisions(mut s: Searcher, pos: Position, l: Limits, t: Transposition) {
        s.tt.set(pos.zobrist(), t);
        assert_eq!(s.search::<1>(&pos, l), s.tt.iter(&pos).collect());
    }

    #[proptest]
    fn search_is_stable(mut s: Searcher, pos: Position, l: Limits) {
        assert_eq!(s.search::<0>(&pos, l), s.search::<0>(&pos, l));
    }

    #[proptest]
    fn search_can_be_limited_by_time(mut s: Searcher, pos: Position, us: u8) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let result = rt.block_on(async {
            let l = Limits::Time(Duration::from_micros(us.into()));
            timeout(Duration::from_millis(1), async { s.search::<1>(&pos, l) }).await
        });

        assert_eq!(result.err(), None);
    }
}
