use crate::chess::{Move, MoveKind, Piece, Position, Role, Zobrist};
use crate::eval::{Eval, Evaluator, Value};
use crate::transposition::{Table, Transposition};
use crate::util::{Timeout, Timer};
use derive_more::{Deref, Neg};
use proptest::prelude::*;
use rayon::{iter::once, prelude::*};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cmp::max_by_key, ops::Range};
use test_strategy::Arbitrary;

mod depth;
mod limits;
mod options;
mod pv;

pub use depth::*;
pub use limits::*;
pub use options::*;
pub use pv::*;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref, Neg)]
struct Score(#[deref] Value, i8);

/// An implementation of [minimax].
///
/// [minimax]: https://www.chessprogramming.org/Searcher
#[derive(Debug, Arbitrary)]
pub struct Searcher {
    evaluator: Evaluator,
    #[strategy(any::<Options>().prop_map(|o|
        ThreadPoolBuilder::new().num_threads(o.threads.get()).build().unwrap()))]
    executor: ThreadPool,
    #[strategy(any::<Options>().prop_map(|o| Table::new(o.hash)))]
    tt: Table,
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new(Evaluator::default())
    }
}

impl Searcher {
    /// Constructs [`Searcher`] with the default [`Options`].
    pub fn new(evaluator: Evaluator) -> Self {
        Self::with_options(evaluator, Options::default())
    }

    /// Constructs [`Searcher`] with the given [`Options`].
    pub fn with_options(evaluator: Evaluator, options: Options) -> Self {
        Searcher {
            evaluator,
            executor: ThreadPoolBuilder::new()
                .num_threads(options.threads.get())
                .build()
                .unwrap(),
            tt: Table::new(options.hash),
        }
    }

    /// Evaluates the [`Position`].
    fn eval(&self, pos: &Position) -> Value {
        self.evaluator.eval(pos)
    }

    /// Probes for a `[Transposition`].
    fn probe(
        &self,
        zobrist: Zobrist,
        bounds: Range<Value>,
        draft: i8,
    ) -> (Option<Transposition>, Value, Value) {
        let transposition = self.tt.get(zobrist);
        let depth = Depth::saturate(draft.max(0) as u8);
        let (alpha, beta) = match transposition.filter(|t| t.depth() >= depth) {
            None => (bounds.start, bounds.end),
            Some(t) => {
                let (lower, upper) = t.bounds().into_inner();

                (
                    bounds.start.clamp(lower, upper),
                    bounds.end.clamp(lower, upper),
                )
            }
        };

        (transposition, alpha, beta)
    }

    /// Records a `[Transposition`].
    fn record(&self, zobrist: Zobrist, bounds: Range<Value>, draft: i8, score: Score, best: Move) {
        let depth = Depth::saturate(draft.max(0) as u8);

        self.tt.set(
            zobrist,
            if *score >= bounds.end {
                Transposition::lower(depth, *score, best)
            } else if *score <= bounds.start {
                Transposition::upper(depth, *score, best)
            } else {
                Transposition::exact(depth, *score, best)
            },
        );
    }

    /// The Static Exchange Evaluation ([SEE]) algorithm.
    ///
    /// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
    fn see<I>(&self, pos: &Position, exchanges: &mut I, bounds: Range<Value>) -> Value
    where
        I: Iterator<Item = Position>,
    {
        assert!(!bounds.is_empty(), "{:?} ≠ ∅", bounds);

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
    ) -> impl DoubleEndedIterator<Item = (Move, Position, Value)> + ExactSizeIterator + '_ {
        pos.moves(kind).map(|(m, next)| {
            let mut exchanges = next.exchanges(m.whither());
            let value = -self.see(&next, &mut exchanges, Value::MIN..Value::MAX);
            (m, next, value)
        })
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(&self, pos: &Position, value: Value, beta: Value, draft: i8) -> Option<i8> {
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
    fn lmp(&self, next: &Position, value: Value, alpha: Value, draft: i8) -> Option<i8> {
        let r = match (alpha - value).get() {
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
    fn aw(&self, pos: &Position, guess: Value, draft: i8, timer: Timer) -> Result<Score, Timeout> {
        let mut w = match draft {
            i8::MIN..=1 => 512,
            2 => 256,
            3 => 128,
            4 => 64,
            _ => 32,
        };

        let mut lower = (guess - w / 2).min(Value::MAX - w);
        let mut upper = (guess + w / 2).max(Value::MIN + w);

        loop {
            w = w.saturating_mul(2);
            match self.pvs(pos, lower..upper, draft, timer)? {
                s if (-lower..Value::MAX).contains(&-s) => lower = *s - w / 2,
                s if (upper..Value::MAX).contains(&s) => upper = *s + w / 2,
                s => break Ok(s),
            }
        }
    }

    /// A [zero-window] wrapper for [`Self::pvs`].
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw(&self, pos: &Position, bound: Value, draft: i8, timer: Timer) -> Result<Score, Timeout> {
        self.pvs(pos, bound..bound + 1, draft, timer)
    }

    /// An implementation of the [PVS] variation of [alpha-beta pruning] algorithm.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn pvs(
        &self,
        pos: &Position,
        bounds: Range<Value>,
        draft: i8,
        timer: Timer,
    ) -> Result<Score, Timeout> {
        assert!(!bounds.is_empty(), "{:?} ≠ ∅", bounds);

        timer.elapsed()?;
        let zobrist = pos.zobrist();
        let (transposition, alpha, beta) = self.probe(zobrist, bounds.clone(), draft);

        if alpha >= beta {
            return Ok(Score(alpha, draft));
        }

        let in_check = pos.is_check();
        let value = match pos.outcome() {
            Some(o) if o.is_draw() => return Ok(Score(Value::ZERO, draft)),
            Some(_) => return Ok(Score(Value::MIN, draft)),
            None => match transposition {
                Some(t) => t.score(),
                None if draft <= 0 => self.eval(pos),
                None => *self.nw(pos, beta - 1, 0, timer)?,
            },
        };

        #[cfg(test)]
        if draft < 0 {
            return Ok(Score(value, draft));
        }

        if !in_check {
            if let Some(d) = self.nmp(pos, value, beta, draft) {
                let mut next = pos.clone();
                next.pass().expect("expected possible pass");
                if d < 0 || *-self.nw(&next, -beta, d, timer)? >= beta {
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

        let mut moves: Vec<_> = self.moves(pos, kind).collect();

        moves.sort_unstable_by_key(|&(m, _, value)| {
            if Some(m) == transposition.map(|t| t.best()) {
                Value::MAX
            } else {
                value
            }
        });

        let (score, best) = match moves.pop() {
            None => return Ok(Score(value, draft)),
            Some((m, next, _)) => {
                let score = -self.pvs(&next, -beta..-alpha, draft - 1, timer)?;

                if *score >= beta {
                    self.record(zobrist, bounds, draft, score, m);
                    return Ok(score);
                }

                (score, m)
            }
        };

        let cutoff = AtomicI16::new(alpha.max(*score).get());

        let (score, best) = moves
            .into_par_iter()
            .rev()
            .map(|(m, next, value)| {
                let mut alpha = Value::new(cutoff.load(Ordering::Relaxed));

                if alpha < beta && !in_check {
                    if let Some(d) = self.lmp(&next, value, alpha, draft) {
                        if d < 0 || *-self.nw(&next, -alpha - 1, d, timer)? < alpha {
                            #[cfg(not(test))]
                            // The late move pruning heuristic is not exact.
                            return Ok(None);
                        }
                    }
                }

                while (beta - alpha).get() > 1 {
                    match -self.nw(&next, -alpha - 1, draft - 1, timer)? {
                        s if *s < alpha => return Ok(Some((s, m))),
                        s => match Value::new(cutoff.fetch_max(s.get(), Ordering::Relaxed)) {
                            _ if *s >= beta => return Ok(Some((s, m))),
                            a if *s >= a => break,
                            a => alpha = a,
                        },
                    }
                }

                if alpha < beta {
                    let score = -self.pvs(&next, -beta..-alpha, draft - 1, timer)?;
                    cutoff.fetch_max(score.get(), Ordering::Relaxed);
                    return Ok(Some((score, m)));
                }

                Ok(None)
            })
            .chain(once(Ok(Some((score, best)))))
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(s, _)| s))))?
            .expect("expected at least one legal move");

        self.record(zobrist, bounds, draft, score, best);

        Ok(score)
    }

    /// Searches for the strongest [variation][`Pv`].
    pub fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let timer = Timer::start(limits.time());
        let mut pv: Pv<N> = self.tt.iter(pos).collect();
        let (mut score, start) = match Option::zip(pv.score(), pv.depth()) {
            Some((s, d)) => (s, d.get() + 1),
            None => {
                self.tt.unset(pos.zobrist());
                (self.eval(pos), 0)
            }
        };

        self.executor.install(|| {
            for depth in start..=limits.depth().get() {
                (score, pv) = match self.aw(pos, score, depth as _, timer) {
                    Ok(s) => (*s, self.tt.iter(pos).collect()),
                    Err(_) => break,
                };
            }
        });

        pv
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prop_assume;
    use std::{cmp::Ordering, time::Duration};
    use test_strategy::proptest;
    use tokio::{runtime, time::timeout};

    fn negamax(evaluator: &Evaluator, pos: &Position, draft: i8) -> Value {
        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Value::ZERO,
            Some(_) => return Value::MIN,
            None => evaluator.eval(pos),
        };

        let kind = match i8::cmp(&0, &draft) {
            Ordering::Greater => return score,
            Ordering::Equal => MoveKind::CAPTURE | MoveKind::PROMOTION,
            Ordering::Less => MoveKind::ANY,
        };

        pos.moves(kind)
            .map(|(_, pos)| -negamax(evaluator, &pos, draft - 1))
            .max()
            .unwrap_or(score)
    }

    #[proptest]
    fn has_is_an_upper_limit_for_table_size(o: Options) {
        let s = Searcher::with_options(Evaluator::default(), o);
        prop_assume!(s.tt.capacity() > 1);
        assert!(s.tt.size() <= o.hash);
    }

    #[proptest]
    #[should_panic]
    fn nw_panics_if_bound_is_too_large(s: Searcher, pos: Position, d: i8) {
        s.nw(&pos, Value::MAX, d, Timer::start(Duration::MAX))?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_not_greater_than_beta(
        s: Searcher,
        pos: Position,
        b: Range<Value>,
        d: i8,
    ) {
        s.pvs(&pos, b.end..b.start, d, Timer::start(Duration::MAX))?;
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        s: Searcher,
        pos: Position,
        #[filter(!#b.is_empty())] b: Range<Value>,
        d: i8,
    ) {
        let timer = Timer::start(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(s.pvs(&pos, b, d, timer), Err(Timeout));
    }

    #[proptest]
    fn pvs_finds_best_score(s: Searcher, pos: Position, d: Depth) {
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            s.pvs(&pos, Value::MIN..Value::MAX, d.get() as _, timer)
                .as_deref(),
            Ok(&negamax(&s.evaluator, &pos, d.get() as _))
        );
    }

    #[proptest]
    fn pvs_does_not_depend_on_table_size(
        e: Evaluator,
        x: Options,
        y: Options,
        pos: Position,
        d: Depth,
    ) {
        let x = Searcher::with_options(e.clone(), x);
        let y = Searcher::with_options(e, y);

        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            x.pvs(&pos, Value::MIN..Value::MAX, d.get() as _, timer)
                .as_deref(),
            y.pvs(&pos, Value::MIN..Value::MAX, d.get() as _, timer)
                .as_deref()
        );
    }

    #[proptest]
    fn aw_finds_best_score(s: Searcher, pos: Position, g: Value, d: Depth) {
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            s.aw(&pos, g, d.get() as _, timer).as_deref(),
            Ok(&negamax(&s.evaluator, &pos, d.get() as _))
        );
    }

    #[proptest]
    fn aw_does_not_depend_on_initial_guess(
        s: Searcher,
        pos: Position,
        g: Value,
        h: Value,
        d: Depth,
    ) {
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            s.aw(&pos, g, d.get() as _, timer).as_deref(),
            s.aw(&pos, h, d.get() as _, timer).as_deref()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(mut s: Searcher, pos: Position, d: Depth) {
        assert_eq!(
            s.search::<4>(&pos, Limits::Depth(d)),
            s.tt.iter(&pos).collect()
        );
    }

    #[proptest]
    fn search_avoids_tt_collisions(
        mut s: Searcher,
        #[by_ref]
        #[filter(#pos.outcome().is_none())]
        pos: Position,
        #[filter(#d > Depth::ZERO)] d: Depth,
        t: Transposition,
    ) {
        s.tt.set(pos.zobrist(), t);
        assert_eq!(s.search::<1>(&pos, Limits::Depth(d)).len(), 1);
    }

    #[proptest]
    fn search_is_stable(mut s: Searcher, pos: Position, d: Depth) {
        assert_eq!(
            s.search::<0>(&pos, Limits::Depth(d)),
            s.search::<0>(&pos, Limits::Depth(d))
        );
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
