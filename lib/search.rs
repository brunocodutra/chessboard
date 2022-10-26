use crate::chess::{Move, MoveKind, Piece, Position, Role, Zobrist};
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
    const MIN_DRAFT: i8 = -2;
    #[cfg(test)]
    const MAX_DRAFT: i8 = 2;

    #[cfg(not(test))]
    const MIN_DRAFT: i8 = -32;
    #[cfg(not(test))]
    const MAX_DRAFT: i8 = Transposition::MAX_DEPTH as i8;

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

    /// Evaluates the [`Position`].
    fn eval(&self, pos: &Position) -> i16 {
        self.evaluator.eval(pos).max(-i16::MAX)
    }

    /// Probes for a `[Transposition`].
    fn probe(
        &self,
        zobrist: Zobrist,
        bounds: &Range<i16>,
        depth: u8,
    ) -> (Option<Transposition>, i16, i16) {
        let transposition = self.tt.get(zobrist);

        let (alpha, beta) = match transposition.filter(|t| t.depth() >= depth) {
            None => (bounds.start, bounds.end),
            #[cfg(test)]
            Some(t) if t.depth() == 0 => (bounds.start, bounds.end),
            Some(t) => {
                let (lower, upper) = t.bounds().into_inner();

                (
                    bounds.start.max(lower).min(upper),
                    bounds.end.min(upper).max(lower),
                )
            }
        };

        (transposition, alpha, beta)
    }

    /// Records a `[Transposition`].
    fn record(&self, zobrist: Zobrist, bounds: &Range<i16>, depth: u8, score: Score, best: Move) {
        self.tt.set(
            zobrist,
            if *score >= bounds.end {
                Transposition::lower(*score, depth, best)
            } else if *score <= bounds.start {
                Transposition::upper(*score, depth, best)
            } else {
                Transposition::exact(*score, depth, best)
            },
        );
    }

    /// The Static Exchange Evaluation ([SEE]) algorithm.
    ///
    /// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
    fn see<I>(&self, pos: &Position, exchanges: &mut I, bounds: Range<i16>) -> i16
    where
        I: Iterator<Item = Position>,
    {
        assert!(!bounds.is_empty(), "{:?} ≠ ∅", bounds);
        assert!(!bounds.contains(&i16::MIN), "{:?} ∌ {}", bounds, i16::MIN);

        let (alpha, beta) = (bounds.start.max(self.eval(pos)), bounds.end);

        if alpha >= beta {
            return beta;
        }

        match exchanges.next() {
            None => alpha,
            Some(next) => -self.see(&next, exchanges, -beta..-alpha),
        }
    }

    fn moves(
        &self,
        pos: &Position,
        kind: MoveKind,
        pv: Option<Move>,
    ) -> Vec<(Move, Position, i16)> {
        let mut moves: Vec<_> = pos
            .moves(kind)
            .map(|(m, next)| {
                let value = if pv == Some(m) {
                    i16::MAX
                } else {
                    let mut exchanges = next.exchanges(m.whither());
                    -self.see(&next, &mut exchanges, -i16::MAX..i16::MAX)
                };

                (m, next, value)
            })
            .collect();

        moves.sort_unstable_by_key(|(_, _, value)| *value);
        moves
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(&self, pos: &Position, value: i16, beta: i16, draft: i8) -> Option<i8> {
        let turn = pos.turn();
        let r = match pos.by_color(turn).len() - pos.by_piece(Piece(turn, Role::Pawn)).len() {
            0..=1 => return None,
            2 => 0,
            3 => 1,
            _ => 2,
        };

        if value > beta {
            Some(draft.saturating_sub(r + 1 + draft.max(0) / 4))
        } else {
            None
        }
    }

    /// An implementation of late move pruning.
    fn lmp(&self, next: &Position, value: i16, alpha: i16, draft: i8) -> Option<i8> {
        let r = match alpha.saturating_sub(value) {
            i16::MIN..=36 => return None,
            37..=108 => 1,
            109..=324 => 2,
            325..=972 => 3,
            _ => 4,
        };

        if !next.is_check() {
            Some(draft.saturating_sub(r + 1))
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
        bounds: Range<i16>,
        draft: i8,
        time: Duration,
        metrics: &MetricsCounters,
    ) -> Result<Score, Timeout> {
        assert!(!bounds.is_empty(), "{:?} ≠ ∅", bounds);
        assert!(!bounds.contains(&i16::MIN), "{:?} ∌ {}", bounds, i16::MIN);

        let mut w = 32;
        if bounds.len() <= 4 * w as usize {
            return self.pvs(pos, bounds, draft, time, metrics);
        }

        let (alpha, beta) = (bounds.start, bounds.end);
        let mut lower = guess.saturating_sub(w / 2).max(alpha).min(beta - w);
        let mut upper = guess.saturating_add(w / 2).max(alpha + w).min(beta);

        loop {
            w = w.saturating_mul(2);
            match self.pvs(pos, lower..upper, draft, time, metrics)? {
                s if (-lower..-alpha).contains(&-s) => lower = s.saturating_sub(w / 2).max(alpha),
                s if (upper..beta).contains(&s) => upper = s.saturating_add(w / 2).min(beta),
                s => break Ok(s),
            }
        }
    }

    /// A [zero-window] wrapper for [`Self::pvs`].
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw(
        &self,
        pos: &Position,
        bound: i16,
        draft: i8,
        time: Duration,
        metrics: &MetricsCounters,
    ) -> Result<Score, Timeout> {
        assert!(bound < i16::MAX, "{} < {}", bound, i16::MAX);
        self.pvs(pos, bound..bound + 1, draft, time, metrics)
    }

    /// An implementation of the [PVS] variation of [alpha-beta pruning] algorithm.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn pvs(
        &self,
        pos: &Position,
        bounds: Range<i16>,
        draft: i8,
        time: Duration,
        metrics: &MetricsCounters,
    ) -> Result<Score, Timeout> {
        assert!(!bounds.is_empty(), "{:?} ≠ ∅", bounds);
        assert!(!bounds.contains(&i16::MIN), "{:?} ∌ {}", bounds, i16::MIN);

        if metrics.time() >= time {
            return Err(Timeout);
        }

        metrics.node();

        let zobrist = pos.zobrist();
        let (transposition, alpha, beta) = self.probe(zobrist, &bounds, draft.max(0) as _);

        if transposition.is_some() {
            metrics.tt_hit();
        }

        if alpha >= beta {
            metrics.tt_cut();
            return Ok(Score(alpha, draft));
        }

        let in_check = pos.is_check();
        let value = match pos.outcome() {
            Some(o) if o.is_draw() => return Ok(Score(0, draft)),
            Some(_) => return Ok(Score(-i16::MAX, draft)),
            None if draft <= 0 => self.eval(pos),
            None => match transposition {
                None => *self.nw(pos, beta - 1, 0, time, metrics)?,
                Some(t) => t.score(),
            },
        };

        if draft <= Self::MIN_DRAFT {
            return Ok(Score(value, draft));
        }

        if !in_check {
            if let Some(d) = self.nmp(pos, value, beta, draft) {
                let mut next = pos.clone();
                next.pass().expect("expected possible pass");
                if d < 0 || *-self.nw(&next, -beta, d, time, metrics)? >= beta {
                    metrics.nm_cut();
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Score(value, draft));
                }
            }
        }

        let kind = if draft <= 0 {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
        };

        let mut moves = self.moves(pos, kind, transposition.map(|t| t.best()));

        let (score, pv) = match moves.pop() {
            None => return Ok(Score(value, draft)),
            Some((m, next, _)) => {
                let score = -self.aw(&next, -value, -beta..-alpha, draft - 1, time, metrics)?;

                if *score >= beta {
                    metrics.pv_cut();
                    self.record(zobrist, &bounds, draft.max(0) as _, score, m);
                    return Ok(score);
                }

                (score, m)
            }
        };

        let cutoff = AtomicI16::new(alpha.max(*score));

        let (score, best) = moves
            .into_par_iter()
            .rev()
            .map(|(m, next, value)| {
                let mut score = Score(-i16::MAX, -i8::MAX);
                let mut alpha = cutoff.load(Ordering::Relaxed);

                if alpha >= beta {
                    return Ok(None);
                }

                if !in_check {
                    if let Some(d) = self.lmp(&next, value, alpha, draft) {
                        if d < 0 || *-self.nw(&next, -alpha - 1, d, time, metrics)? < alpha {
                            #[cfg(not(test))]
                            // The late move pruning heuristic is not exact.
                            return Ok(Some((score, m)));
                        } else {
                            alpha = cutoff.load(Ordering::Relaxed);
                        }
                    }
                }

                while *score < alpha && alpha < beta {
                    score = -self.nw(&next, -alpha - 1, draft - 1, time, metrics)?;

                    if *score < alpha {
                        break;
                    } else {
                        alpha = cutoff.fetch_max(*score, Ordering::Relaxed).max(*score);
                    }
                }

                if (alpha..beta).contains(&score) {
                    score = -self.aw(&next, -alpha, -beta..-alpha, draft - 1, time, metrics)?;
                    cutoff.fetch_max(*score, Ordering::Relaxed);
                }

                Ok(Some((score, m)))
            })
            .chain(once(Ok(Some((score, pv)))))
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(s, _)| s))))?
            .expect("expected at least one legal move");

        self.record(zobrist, &bounds, draft.max(0) as _, score, best);

        Ok(score)
    }

    /// Searches for the strongest [variation][`Pv`].
    pub fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let mut pv: Pv<N> = self.tt.iter(pos).collect();

        let (mut score, start) = Option::zip(pv.score(), pv.depth()).unwrap_or_else(|| {
            self.tt.unset(pos.zobrist());
            (self.eval(pos), 0)
        });

        let mut metrics = Metrics::default();
        let mut counters = MetricsCounters::default();
        for depth in start..=limits.depth().min(Self::MAX_DRAFT as u8) {
            let bounds = -i16::MAX..i16::MAX;
            (score, pv) = match self.aw(pos, score, bounds, depth as i8, limits.time(), &counters) {
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
        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return 0,
            Some(_) => return -i16::MAX,
            None => evaluator.eval(pos).max(-i16::MAX),
        };

        let kind = if draft <= Searcher::MIN_DRAFT {
            return score;
        } else if draft <= 0 {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
        };

        pos.moves(kind)
            .map(|(_, pos)| -negamax(evaluator, &pos, draft - 1))
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
    fn nw_panics_if_bound_is_too_large(s: Searcher, pos: Position, d: i8) {
        let metrics = MetricsCounters::default();
        s.nw(&pos, i16::MAX, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_too_small(s: Searcher, pos: Position, b: i16, d: i8) {
        let metrics = MetricsCounters::default();
        s.pvs(&pos, i16::MIN..b, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_not_greater_than_beta(
        s: Searcher,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(..=#a)] b: i16,
        d: i8,
    ) {
        let metrics = MetricsCounters::default();
        s.pvs(&pos, a..b, d, Duration::MAX, &metrics)?;
    }

    #[proptest]
    fn pvs_evaluates_position_if_draft_is_lower_than_minimum(
        s: Searcher,
        #[by_ref]
        #[filter(#pos.outcome().is_none())]
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(#a + 1..)] b: i16,
        #[strategy(i8::MIN..=Searcher::MIN_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.pvs(&pos, a..b, d, Duration::MAX, &metrics),
            Ok(Score(s.eval(&pos), d))
        );
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        s: Searcher,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] a: i16,
        #[strategy(#a+1..)] b: i16,
        d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(s.pvs(&pos, a..b, d, Duration::ZERO, &metrics), Err(Timeout));
    }

    #[proptest]
    fn pvs_finds_best_score(
        s: Searcher,
        pos: Position,
        #[strategy(0..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.pvs(&pos, -i16::MAX..i16::MAX, d, Duration::MAX, &metrics)
                .as_deref(),
            Ok(&negamax(&s.evaluator, &pos, d))
        );
    }

    #[proptest]
    fn pvs_does_not_depend_on_table_size(
        e: Evaluator,
        x: Options,
        y: Options,
        pos: Position,
        #[strategy(0..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let x = Searcher::with_options(e.clone(), x);
        let y = Searcher::with_options(e, y);

        let xc = MetricsCounters::default();
        let yc = MetricsCounters::default();

        assert_eq!(
            x.pvs(&pos, -i16::MAX..i16::MAX, d, Duration::MAX, &xc)
                .as_deref(),
            y.pvs(&pos, -i16::MAX..i16::MAX, d, Duration::MAX, &yc)
                .as_deref()
        );
    }

    #[proptest]
    fn aw_finds_best_score(
        s: Searcher,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] g: i16,
        #[strategy(0..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.aw(&pos, g, -i16::MAX..i16::MAX, d, Duration::MAX, &metrics)
                .as_deref(),
            Ok(&negamax(&s.evaluator, &pos, d))
        );
    }

    #[proptest]
    fn aw_does_not_depend_on_initial_guess(
        s: Searcher,
        pos: Position,
        #[strategy(-i16::MAX..i16::MAX)] g: i16,
        #[strategy(-i16::MAX..i16::MAX)] h: i16,
        #[strategy(0..=Searcher::MAX_DRAFT)] d: i8,
    ) {
        let metrics = MetricsCounters::default();
        assert_eq!(
            s.aw(&pos, g, -i16::MAX..i16::MAX, d, Duration::MAX, &metrics)
                .as_deref(),
            s.aw(&pos, h, -i16::MAX..i16::MAX, d, Duration::MAX, &metrics)
                .as_deref()
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
