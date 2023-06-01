use crate::chess::{Move, MoveKind, Piece, Position, Role, Zobrist};
use crate::nnue::Evaluator;
use crate::search::{Depth, Limits, Line, Options, Ply, Pv, Score, Value};
use crate::search::{Transposition, TranspositionTable};
use crate::util::{Timeout, Timer};
use arrayvec::ArrayVec;
use derive_more::{Deref, Neg};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cmp::max_by_key, ops::Range, time::Duration};
use test_strategy::Arbitrary;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref, Neg)]
struct ScoreWithTempo(#[deref] Score, Ply);

impl ScoreWithTempo {
    fn new(score: Score, depth: Depth, ply: Ply) -> Self {
        ScoreWithTempo(score, -ply + depth)
    }

    fn zero(depth: Depth, ply: Ply) -> Self {
        ScoreWithTempo::new(Score::new(0), depth, ply)
    }

    fn lower(depth: Depth, ply: Ply) -> Self {
        ScoreWithTempo::new(Score::LOWER.normalize(ply), depth, ply)
    }
}

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
    #[cfg(not(test))]
    const MAX_PLY: i8 = Ply::UPPER.get() / 2;

    #[cfg(test)]
    const MAX_PLY: i8 = 3;

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
        let transposition = self.tt.get(zobrist);
        let (lower, upper) = match transposition {
            Some(t) if t.depth() >= depth - ply => t.bounds().into_inner(),
            _ => (Score::LOWER, Score::UPPER),
        };

        // One can't mate in 0 plies!
        let min = lower.min(Score::UPPER - 1).normalize(ply);
        let max = upper.min(Score::UPPER - 1).normalize(ply);
        let (alpha, beta) = (bounds.start, bounds.end);
        (transposition, alpha.clamp(min, max), beta.clamp(min, max))
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

    /// A [zero-window] wrapper for [`Self::pvs`].
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw(
        &self,
        pos: &Evaluator,
        beta: Score,
        depth: Depth,
        ply: Ply,
        timer: Timer,
    ) -> Result<ScoreWithTempo, Timeout> {
        self.pvs(pos, beta - 1..beta, depth, ply, timer)
    }

    /// An implementation of the [PVS] variation of [alpha-beta pruning] algorithm.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn pvs(
        &self,
        pos: &Evaluator,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        timer: Timer,
    ) -> Result<ScoreWithTempo, Timeout> {
        assert!(!bounds.is_empty(), "{bounds:?} ≠ ∅");

        timer.elapsed()?;
        let in_check = pos.is_check();
        let zobrist = match pos.outcome() {
            Some(o) if o.is_draw() => return Ok(ScoreWithTempo::zero(depth, ply)),
            Some(_) => return Ok(ScoreWithTempo::lower(depth, ply)),
            None => pos.zobrist(),
        };

        let (transposition, alpha, beta) = self.probe(zobrist, bounds.clone(), depth, ply);

        if alpha >= beta {
            return Ok(ScoreWithTempo::new(alpha, depth, ply));
        }

        let score = match transposition {
            Some(t) => t.score().normalize(ply),
            None if ply < depth => *self.nw(pos, beta, ply.cast(), ply, timer)?,
            None => pos.value().cast(),
        };

        let kind = if ply < depth {
            MoveKind::ANY
        } else if ply < Searcher::MAX_PLY {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            return Ok(ScoreWithTempo::new(score, depth, ply));
        };

        if !in_check {
            if let Some(d) = self.nmp(pos, score, beta, depth) {
                let mut next = pos.clone();
                next.pass().expect("expected possible pass");
                if d <= ply || *-self.nw(&next, -beta + 1, d, ply + 1, timer)? >= beta {
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(ScoreWithTempo::new(score, depth, ply));
                }
            }
        }

        let mut moves = ArrayVec::<_, 256>::from_iter(pos.moves(kind).map(|m| {
            let mut next = pos.clone();
            next.play(*m).expect("expected legal move");
            let value = -next.see(m.whither(), Value::LOWER..Value::UPPER);
            (*m, value.cast())
        }));

        moves.sort_unstable_by_key(|&(m, guess)| {
            if Some(m) == transposition.map(|t| t.best()) {
                Score::UPPER
            } else {
                guess
            }
        });

        let (score, best) = match moves.pop() {
            None => return Ok(ScoreWithTempo::new(score, depth, ply)),
            Some((m, _)) => {
                let mut next = pos.clone();
                next.play(m).expect("expected legal move");
                let score = -self.pvs(&next, -beta..-alpha, depth, ply + 1, timer)?;

                if *score >= beta {
                    self.record(zobrist, bounds, depth, ply, *score, m);
                    return Ok(score);
                }

                (score, m)
            }
        };

        let cutoff = AtomicI16::new(alpha.max(*score).get());

        let (score, best) = moves
            .into_par_iter()
            .rev()
            .map(|&(m, guess)| {
                let mut alpha = Score::new(cutoff.load(Ordering::Relaxed));

                if alpha >= beta {
                    return Ok(None);
                }

                let mut next = pos.clone();
                next.play(m).expect("expected legal move");

                if !in_check {
                    if let Some(d) = self.lmp(&next, guess, alpha.cast(), depth) {
                        if d <= ply || *-self.nw(&next, -alpha, d, ply + 1, timer)? < alpha {
                            #[cfg(not(test))]
                            // The late move pruning heuristic is not exact.
                            return Ok(None);
                        }
                    }
                }

                loop {
                    match -self.nw(&next, -alpha, depth, ply + 1, timer)? {
                        s if *s < alpha => return Ok(Some((s, m))),
                        s => match Score::new(cutoff.fetch_max(s.get(), Ordering::Relaxed)) {
                            _ if *s >= beta => return Ok(Some((s, m))),
                            a if a >= beta => return Ok(None),
                            a if *s < a => alpha = a,
                            a => {
                                let s = -self.pvs(&next, -beta..-a, depth, ply + 1, timer)?;
                                cutoff.fetch_max(s.get(), Ordering::Relaxed);
                                return Ok(Some((s, m)));
                            }
                        },
                    }
                }
            })
            .chain([Ok(Some((score, best)))])
            .try_reduce(|| None, |a, b| Ok(max_by_key(a, b, |x| x.map(|(s, _)| s))))?
            .expect("expected at least one legal move");

        self.record(zobrist, bounds, depth, ply, *score, best);

        Ok(score)
    }

    fn time_to_search(&self, pos: &Position, limits: Limits) -> Duration {
        let (clock, inc) = match limits {
            Limits::Clock(c, i) => (c, i),
            _ => return limits.time(),
        };

        let cap = clock.mul_f64(0.8);
        let excess = clock.saturating_sub(inc);
        let moves_left = 45 - (pos.fullmoves().get() - 1).min(20);
        inc.saturating_add(excess / moves_left).min(cap)
    }

    /// Searches for the [principal variation][`Pv`].
    pub fn search<const N: usize>(&mut self, pos: &Position, limits: Limits) -> Pv<N> {
        let pos = Evaluator::borrow(pos);
        let timer = Timer::start(self.time_to_search(&pos, limits));

        if self.tt.iter(&pos).next().is_none() {
            self.tt.unset(pos.zobrist());
        };

        self.executor.install(|| {
            let mut pv = Pv::new(Depth::new(0), Score::new(0), Line::empty());

            'id: for d in 0..=limits.depth().get() {
                let mut w: i16 = 32;
                let mut lower = (pv.score() - w / 2).min(Score::UPPER - w);
                let mut upper = (pv.score() + w / 2).max(Score::LOWER + w);

                pv = 'aw: loop {
                    let timer = if pv.is_empty() {
                        // Ignore time limits until some pv is found.
                        Timer::start(Duration::MAX)
                    } else {
                        timer
                    };

                    let depth = Depth::new(d);
                    let score = match self.pvs(&pos, lower..upper, depth, Ply::new(0), timer) {
                        Err(_) => break 'id,
                        Ok(s) => *s,
                    };

                    w = w.saturating_mul(2);

                    match score {
                        s if (-lower..Score::UPPER).contains(&-s) => lower = s - w / 2,
                        s if (upper..Score::UPPER).contains(&s) => upper = s + w / 2,
                        _ => break 'aw Pv::new(depth, score, self.tt.line(&pos).collect()),
                    }

                    if score >= pv.score() {
                        pv = Pv::new(depth, score, self.tt.line(&pos).collect());
                    }
                };
            }

            pv
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assume, sample::Selector};
    use std::time::Duration;
    use test_strategy::proptest;
    use tokio::{runtime, time::timeout};

    fn negamax(pos: &Evaluator, depth: Depth, ply: Ply) -> Score {
        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Score::new(0),
            Some(_) => return Score::LOWER.normalize(ply),
            None => pos.value().cast(),
        };

        let kind = if ply < depth {
            MoveKind::ANY
        } else if ply < Searcher::MAX_PLY {
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
        s.nw(&pos, Score::LOWER, d, p, Timer::start(Duration::MAX))?;
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
        s.pvs(&pos, b.end..b.start, d, p, Timer::start(Duration::MAX))?;
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
        assert_eq!(s.pvs(&pos, b, d, p, timer), Err(Timeout));
    }

    #[proptest]
    fn pvs_returns_balanced_score_if_game_ends_in_a_draw(
        s: Searcher,
        #[filter(#pos.outcome().is_some_and(|o| o.is_draw()))] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        assert_eq!(s.pvs(&pos, b, d, p, timer), Ok(ScoreWithTempo::zero(d, p)));
    }

    #[proptest]
    fn pvs_returns_lowest_score_if_game_ends_in_checkmate(
        s: Searcher,
        #[filter(#pos.outcome().is_some_and(|o| o.is_decisive()))] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        assert_eq!(s.pvs(&pos, b, d, p, timer), Ok(ScoreWithTempo::lower(d, p)));
    }

    #[proptest]
    fn pvs_returns_transposition_score_if_exact(
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

        assert_eq!(
            s.pvs(&pos, b, d, p, timer),
            Ok(ScoreWithTempo::new(sc, d, p))
        );
    }

    #[proptest]
    fn pvs_finds_best_score(
        s: Searcher,
        pos: Position,
        #[filter((1..=3).contains(&#d.get()))] d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);
        let bounds = Score::LOWER..Score::UPPER;

        assert_eq!(
            s.pvs(&pos, bounds, d, p, timer).as_deref(),
            Ok(&negamax(&pos, d, p))
        );
    }

    #[proptest]
    fn pvs_does_not_depend_on_table_size(
        x: Options,
        y: Options,
        pos: Position,
        #[filter((1..=3).contains(&#d.get()))] d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let x = Searcher::with_options(x);
        let y = Searcher::with_options(y);

        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::MAX);

        assert_eq!(
            x.pvs(&pos, Score::LOWER..Score::UPPER, d, p, timer),
            y.pvs(&pos, Score::LOWER..Score::UPPER, d, p, timer)
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(
        mut s: Searcher,
        pos: Position,
        #[filter((1..=3).contains(&#d.get()))] d: Depth,
    ) {
        assert_eq!(
            *s.search::<3>(&pos, Limits::Depth(d)),
            s.tt.line(&pos).collect()
        );
    }

    #[proptest]
    fn search_avoids_tt_collisions(
        mut s: Searcher,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[filter((1..=3).contains(&#d.get()))] d: Depth,
        t: Transposition,
    ) {
        s.tt.set(pos.zobrist(), t);
        assert!(!s.search::<3>(&pos, Limits::Depth(d)).is_empty());
    }

    #[proptest]
    fn search_is_stable(
        mut s: Searcher,
        pos: Position,
        #[filter((1..=3).contains(&#d.get()))] d: Depth,
    ) {
        assert_eq!(
            s.search::<1>(&pos, Limits::Depth(d)),
            s.search::<1>(&pos, Limits::Depth(d))
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

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut s: Searcher,
        #[filter(#pos.outcome().is_none())] pos: Position,
    ) {
        assert!(!s.search::<1>(&pos, Limits::Time(Duration::ZERO)).is_empty());
    }
}
