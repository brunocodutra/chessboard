use crate::chess::{Move, MoveKind, Piece, Position, Role, Zobrist};
use crate::nnue::Evaluator;
use crate::search::{Depth, Limits, Options, Ply, Pv, Score, Value};
use crate::search::{Transposition, TranspositionTable};
use crate::util::{Timeout, Timer};
use arrayvec::ArrayVec;
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cmp::max, ops::Range, time::Duration};
use test_strategy::Arbitrary;

/// An implementation of [minimax].
///
/// [minimax]: https://www.chessprogramming.org/Searcher
#[derive(Debug, Arbitrary)]
pub struct Searcher {
    #[map(|o: Options| ThreadPoolBuilder::new().num_threads(o.threads.get()).build().unwrap())]
    executor: ThreadPool,
    #[map(|o: Options| TranspositionTable::new(o.hash))]
    tt: TranspositionTable,
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher {
    /// Constructs [`Searcher`] with the default [`Options`].
    pub fn new() -> Self {
        Self::with_options(Options::default())
    }

    /// Constructs [`Searcher`] with the given [`Options`].
    pub fn with_options(options: Options) -> Self {
        Searcher {
            executor: ThreadPoolBuilder::new()
                .num_threads(options.threads.get())
                .build()
                .unwrap(),
            tt: TranspositionTable::new(options.hash),
        }
    }

    /// Probes for a `[Transposition`].
    fn probe(
        &self,
        zobrist: Zobrist,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
    ) -> (Option<Transposition>, Score, Score) {
        let tpos = self.tt.get(zobrist);
        let (lower, upper) = match tpos {
            Some(t) if t.depth() >= depth - ply => t.bounds().into_inner(),
            _ => (Score::LOWER, Score::UPPER),
        };

        // One can't mate in 0 plies!
        let min = lower.min(Score::UPPER - 1).normalize(ply);
        let max = upper.min(Score::UPPER - 1).normalize(ply);
        let (alpha, beta) = (bounds.start, bounds.end);
        (tpos, alpha.clamp(min, max), beta.clamp(min, max))
    }

    /// Records a `[Transposition`].
    fn record(
        &self,
        zobrist: Zobrist,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        score: Score,
        best: Move,
    ) {
        self.tt.set(
            zobrist,
            if score >= bounds.end {
                Transposition::lower(depth - ply, score.normalize(-ply), best)
            } else if score <= bounds.start {
                Transposition::upper(depth - ply, score.normalize(-ply), best)
            } else {
                Transposition::exact(depth - ply, score.normalize(-ply), best)
            },
        );
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(&self, pos: &Position, guess: Score, beta: Score, depth: Depth) -> Option<Depth> {
        let turn = pos.turn();
        let r = match pos.by_color(turn).len() - pos.by_piece(Piece(turn, Role::Pawn)).len() {
            ..=1 => return None,
            2 => 0,
            3 => 1,
            _ => 2,
        };

        if guess > beta {
            Some(depth - r)
        } else {
            None
        }
    }

    /// An implementation of late move pruning.
    fn lmp(&self, next: &Position, guess: Score, alpha: Score, depth: Depth) -> Option<Depth> {
        let r = match (alpha - guess).get() {
            ..=90 => return None,
            91..=270 => 1,
            271..=810 => 2,
            811..=2430 => 3,
            2431..=7290 => 4,
            _ => 5,
        };

        if !next.is_check() {
            Some(depth - r)
        } else {
            None
        }
    }

    /// A [zero-window] search wrapper for [`Self::pvs`].
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw<const N: usize>(
        &self,
        pos: &Evaluator,
        beta: Score,
        depth: Depth,
        ply: Ply,
        timer: Timer,
    ) -> Result<Pv<N>, Timeout> {
        self.pvs(pos, beta - 1..beta, depth, ply, timer)
    }

    /// An implementation of the [PVS] variation of [alpha-beta pruning] algorithm.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn pvs<const N: usize>(
        &self,
        pos: &Evaluator,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        timer: Timer,
    ) -> Result<Pv<N>, Timeout> {
        assert!(!bounds.is_empty(), "{bounds:?} ≠ ∅");

        timer.elapsed()?;
        let in_check = pos.is_check();
        let zobrist = match pos.outcome() {
            Some(o) if o.is_draw() => return Ok(Pv::new(Score::new(0), depth, [])),
            Some(_) => return Ok(Pv::new(Score::LOWER.normalize(ply), depth, [])),
            None => pos.zobrist(),
        };

        let (tpos, alpha, beta) = self.probe(zobrist, bounds.clone(), depth, ply);

        if alpha >= beta {
            return match tpos {
                Some(t) => Ok(Pv::new(t.score().normalize(ply), depth, [t.best()])),
                None => Ok(Pv::new(alpha, depth, [])),
            };
        }

        let score = match tpos {
            Some(t) => t.score().normalize(ply),
            None if ply < depth => self.nw::<1>(pos, beta, ply.cast(), ply, timer)?.score(),
            None => pos.value().cast(),
        };

        if ply >= Ply::UPPER {
            return Ok(Pv::new(score, depth, []));
        } else if !in_check {
            if let Some(d) = self.nmp(pos, score, beta, depth) {
                let mut next = pos.clone();
                next.pass().expect("expected possible pass");
                if d <= ply || -self.nw::<1>(&next, -beta + 1, d, ply + 1, timer)? >= beta {
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Pv::new(score, depth, []));
                }
            }
        }

        let kind = if ply < depth {
            MoveKind::ANY
        } else {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        };

        let mut moves = ArrayVec::<_, 256>::from_iter(pos.moves(kind).map(|m| {
            let mut next = pos.clone();
            next.play(*m).expect("expected legal move");
            let value = -next.see(m.whither(), Value::LOWER..Value::UPPER);
            (*m, value.cast())
        }));

        moves.sort_unstable_by_key(|&(m, guess)| {
            if Some(m) == tpos.map(|t| t.best()) {
                Score::UPPER
            } else {
                guess
            }
        });

        let best = match moves.pop() {
            None => return Ok(Pv::new(score, depth, [])),
            Some((m, _)) => {
                let mut next = pos.clone();
                next.play(m).expect("expected legal move");
                let pv = -self.pvs(&next, -beta..-alpha, depth, ply + 1, timer)?;

                if pv >= beta {
                    self.record(zobrist, bounds, depth, ply, pv.score(), m);
                    return Ok(pv.shift(m));
                }

                pv.shift(m)
            }
        };

        let cutoff = AtomicI16::new(best.score().max(alpha).get());

        let (best, _) = moves
            .into_par_iter()
            .copied()
            .enumerate()
            .rev()
            .map(|(n, (m, guess))| {
                let alpha = Score::new(cutoff.load(Ordering::Relaxed));

                if alpha >= beta {
                    return Ok(None);
                }

                let mut next = pos.clone();
                next.play(m).expect("expected legal move");

                if !in_check {
                    if let Some(d) = self.lmp(&next, guess, alpha.cast(), depth) {
                        if d <= ply || -self.nw::<1>(&next, -alpha, d, ply + 1, timer)? < alpha {
                            #[cfg(not(test))]
                            // The late move pruning heuristic is not exact.
                            return Ok(None);
                        }
                    }
                }

                let pv = match -self.nw(&next, -alpha, depth, ply + 1, timer)? {
                    pv if pv < alpha => return Ok(Some((pv.shift(m), n))),
                    _ => -self.pvs(&next, -beta..-alpha, depth, ply + 1, timer)?,
                };

                cutoff.fetch_max(pv.score().get(), Ordering::Relaxed);
                Ok(Some((pv.shift(m), n)))
            })
            .chain([Ok(Some((best, usize::MAX)))])
            .try_reduce(|| None, |a, b| Ok(max(a, b)))?
            .expect("expected at least one principal variation");

        self.record(zobrist, bounds, depth, ply, best.score(), best[0]);

        Ok(best)
    }

    /// An implementation of [aspiration windows] with [iterative deepening].
    ///
    /// [aspiration windows]: https://www.chessprogramming.org/Aspiration_Windows
    /// [iterative deepening]: https://www.chessprogramming.org/Iterative_Deepening
    fn aw<const N: usize>(&self, pos: &Position, depth: Depth, timer: Timer) -> Pv<N> {
        let mut best = Pv::new(Score::new(0), Depth::new(0), []);

        'id: for d in 0..=depth.get() {
            let mut w: i16 = 32;
            let mut lower = (best.score() - w / 2).min(Score::UPPER - w);
            let mut upper = (best.score() + w / 2).max(Score::LOWER + w);

            best = 'aw: loop {
                let timer = if best.is_empty() {
                    // Ignore time limits until some pv is found.
                    Timer::start(Duration::MAX)
                } else {
                    timer
                };

                let depth = Depth::new(d);
                let pos = Evaluator::borrow(pos);
                let pv = match self.pvs(&pos, lower..upper, depth, Ply::new(0), timer) {
                    Err(_) => break 'id,
                    Ok(pv) => pv,
                };

                w = w.saturating_mul(2);

                match pv.score() {
                    s if (-lower..Score::UPPER).contains(&-s) => lower = s - w / 2,
                    s if (upper..Score::UPPER).contains(&s) => upper = s + w / 2,
                    _ => break 'aw pv,
                }

                best = best.max(pv);
            };
        }

        best
    }

    fn time_to_search(&self, pos: &Position, limits: Limits) -> Duration {
        let (clock, inc) = match limits {
            Limits::Clock(c, i) => (c, i),
            _ => return limits.time(),
        };

        let cap = clock.mul_f32(0.8);
        let excess = clock.saturating_sub(inc);
        let scale = 400 / pos.fullmoves().get().min(40);
        inc.saturating_add(excess / scale).min(cap)
    }

    /// Searches for the [principal variation][`Pv`].
    pub fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let depth = limits.depth();
        let timer = Timer::start(self.time_to_search(pos, limits));
        self.executor.install(|| self.aw(pos, depth, timer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assume, sample::Selector};
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::time::timeout;

    fn negamax(pos: &Evaluator, depth: Depth, ply: Ply) -> Score {
        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Score::new(0),
            Some(_) => return Score::LOWER.normalize(ply),
            None => pos.value().cast(),
        };

        let kind = if ply < depth {
            MoveKind::ANY
        } else if ply < Ply::UPPER {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            return score;
        };

        pos.moves(kind)
            .map(|m| {
                let mut next = pos.clone();
                next.play(*m).unwrap();
                -negamax(&next, depth, ply + 1)
            })
            .max()
            .unwrap_or(score)
    }

    #[proptest]
    fn has_is_an_upper_limit_for_table_size(o: Options) {
        let s = Searcher::with_options(o);
        prop_assume!(s.tt.capacity() > 1);
        assert!(s.tt.size() <= o.hash);
    }

    #[proptest]
    #[should_panic]
    fn nw_panics_if_beta_is_too_small(s: Searcher, pos: Position, d: Depth, p: Ply) {
        let pos = Evaluator::borrow(&pos);
        s.nw::<1>(&pos, Score::LOWER, d, p, Timer::start(Duration::MAX))?;
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_not_greater_than_beta(
        s: Searcher,
        pos: Position,
        b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        s.pvs::<1>(&pos, b.end..b.start, d, p, Timer::start(Duration::MAX))?;
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        s: Searcher,
        pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(s.pvs::<1>(&pos, b, d, p, timer), Err(Timeout));
    }

    #[proptest]
    fn pvs_returns_drawn_score_if_game_ends_in_a_draw(
        s: Searcher,
        #[filter(#pos.outcome().is_some_and(|o| o.is_draw()))] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            s.pvs(&pos, b, d, p, timer),
            Ok(Pv::<1>::new(Score::new(0), d, []))
        );
    }

    #[proptest]
    fn pvs_returns_lost_score_if_game_ends_in_checkmate(
        s: Searcher,
        #[filter(#pos.outcome().is_some_and(|o| o.is_decisive()))] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            s.pvs(&pos, b, d, p, timer),
            Ok(Pv::<1>::new(Score::LOWER.normalize(p), d, []))
        );
    }

    #[proptest]
    fn pvs_returns_transposition_if_exact(
        #[by_ref]
        #[filter(#s.tt.capacity() > 0)]
        s: Searcher,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#sc.mate().is_none())] sc: Score,
        selector: Selector,
    ) {
        let m = *selector.select(pos.moves(MoveKind::ANY));
        s.tt.set(pos.zobrist(), Transposition::exact(d, sc, m));

        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        assert_eq!(s.pvs(&pos, b, d, p, timer), Ok(Pv::<1>::new(sc, d, [m])));
    }

    #[proptest]
    fn pvs_returns_pv_of_the_given_depth(
        s: Searcher,
        pos: Position,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        let bounds = Score::LOWER..Score::UPPER;

        assert_eq!(s.pvs::<1>(&pos, bounds, d, p, timer)?.depth(), d);
    }

    #[proptest]
    fn pvs_finds_best_score(s: Searcher, pos: Position, d: Depth, #[filter(#p >= 0)] p: Ply) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        let bounds = Score::LOWER..Score::UPPER;

        assert_eq!(s.pvs::<1>(&pos, bounds, d, p, timer)?, negamax(&pos, d, p));
    }

    #[proptest]
    fn pvs_does_not_depend_on_configuration(
        x: Options,
        y: Options,
        pos: Position,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let x = Searcher::with_options(x);
        let y = Searcher::with_options(y);

        let pos = Evaluator::borrow(&pos);
        let bounds = Score::LOWER..Score::UPPER;
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            x.pvs::<1>(&pos, bounds.clone(), d, p, timer)?.score(),
            y.pvs::<1>(&pos, bounds, d, p, timer)?.score()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(mut s: Searcher, pos: Position, d: Depth) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        let bounds = Score::LOWER..Score::UPPER;

        assert_eq!(
            s.search::<3>(&pos, Limits::Depth(d)).score(),
            s.pvs::<3>(&pos, bounds, d, Ply::new(0), timer)?.score()
        );
    }

    #[proptest]
    fn search_is_stable(mut s: Searcher, pos: Position, d: Depth) {
        assert_eq!(
            s.search::<3>(&pos, Limits::Depth(d)).score(),
            s.search::<3>(&pos, Limits::Depth(d)).score()
        );
    }

    #[proptest(async = "tokio")]
    async fn search_can_be_limited_by_time(mut s: Searcher, pos: Position, us: u8) {
        let l = Limits::Time(Duration::from_micros(us.into()));
        let r = timeout(Duration::from_millis(1), async { s.search::<1>(&pos, l) }).await;
        assert_eq!(r.err(), None);
    }

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut s: Searcher,
        #[filter(#pos.outcome().is_none())] pos: Position,
    ) {
        assert!(!s.search::<1>(&pos, Limits::Time(Duration::ZERO)).is_empty());
    }
}
