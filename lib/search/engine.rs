use crate::chess::{Move, MoveKind, Piece, Position, Role, Zobrist};
use crate::nnue::Evaluator;
use crate::search::{Depth, Limits, Options, Ply, Pv, Score, Value};
use crate::search::{Transposition, TranspositionTable};
use crate::util::{Timeout, Timer};
use arrayvec::ArrayVec;
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cmp::max, ops::Range, time::Duration};

/// A chess engine.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Engine {
    #[cfg_attr(test, map(|o: Options| ThreadPoolBuilder::new().num_threads(o.threads.get()).build().unwrap()))]
    executor: ThreadPool,
    #[cfg_attr(test, map(|o: Options| TranspositionTable::new(o.hash)))]
    tt: TranspositionTable,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Initializes the engine with the default [`Options`].
    pub fn new() -> Self {
        Self::with_options(Options::default())
    }

    /// Initializes the engine with the given [`Options`].
    pub fn with_options(options: Options) -> Self {
        Engine {
            executor: ThreadPoolBuilder::new()
                .num_threads(options.threads.get())
                .build()
                .unwrap(),
            tt: TranspositionTable::new(options.hash),
        }
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

    /// An implementation of [mate distance pruning].
    ///
    /// [mate distance pruning]: https://www.chessprogramming.org/Mate_Distance_Pruning
    fn mdp(&self, ply: Ply, bounds: &Range<Score>) -> (Score, Score) {
        let lower = Score::LOWER.normalize(ply);
        let upper = (Score::UPPER - 1).normalize(ply); // One can't mate in 0 plies!
        let alpha = bounds.start.clamp(lower, upper);
        let beta = bounds.end.clamp(lower, upper);
        (alpha, beta)
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(&self, pos: &Position, guess: Score, beta: Score, depth: Depth) -> Option<Depth> {
        let turn = pos.turn();
        let r = match pos.by_color(turn).len() - pos.by_piece(Piece(turn, Role::Pawn)).len() {
            ..=1 => return None,
            2 => 2,
            3 => 3,
            _ => 4,
        };

        if guess > beta {
            Some(depth - r)
        } else {
            None
        }
    }

    /// An implementation of [late move pruning].
    ///
    /// [late move pruning]: https://www.chessprogramming.org/Late_Move_Reductions
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

    /// A [zero-window] search wrapper for [`Self::ns`].
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
        self.ns(pos, beta - 1..beta, depth, ply, timer)
    }

    /// An implementation of the [negascout] variation of [alpha-beta pruning] algorithm.
    ///
    /// [negascout]: https://www.chessprogramming.org/NegaScout
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn ns<const N: usize>(
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
            Some(o) if o.is_draw() => return Ok(Pv::new(Score::new(0), [])),
            Some(_) => return Ok(Pv::new(Score::LOWER.normalize(ply), [])),
            None => pos.zobrist(),
        };

        let (alpha, beta) = self.mdp(ply, &bounds);
        let is_pv = alpha + 1 < beta;

        if alpha >= beta {
            return Ok(Pv::new(alpha, []));
        }

        let tpos = self.tt.get(zobrist);

        if !is_pv {
            if let Some(t) = tpos {
                if t.depth() >= depth - ply {
                    let (lower, upper) = t.bounds().into_inner();
                    if lower == upper || upper <= alpha || lower >= beta {
                        return Ok(Pv::new(t.score().normalize(ply), [t.best()]));
                    }
                }
            }
        }

        let depth = match tpos {
            #[cfg(not(test))]
            // Reductions are not exact.
            None => depth - 1,
            _ => depth,
        };

        let score = match tpos {
            _ if ply >= depth => pos.value().cast(),
            Some(t) => t.score().normalize(ply),
            None => self.nw::<1>(pos, beta, ply.cast(), ply, timer)?.score(),
        };

        if ply >= Ply::UPPER {
            return Ok(Pv::new(score, []));
        } else if !in_check {
            if let Some(d) = self.nmp(pos, score, beta, depth) {
                let mut next = pos.clone();
                next.pass().expect("expected possible pass");
                if d <= ply || -self.nw::<1>(&next, -beta + 1, d, ply + 1, timer)? >= beta {
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Pv::new(score, []));
                }
            }
        }

        let kind = if ply >= depth && !in_check {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
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
            None => return Ok(Pv::new(score, [])),
            Some((m, _)) => {
                let mut next = pos.clone();
                next.play(m).expect("expected legal move");
                let pv = -self.ns(&next, -beta..-alpha, depth, ply + 1, timer)?;

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
            .with_max_len(1)
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
                    if let Some(d) = self.lmp(&next, guess, alpha, depth) {
                        if d <= ply || -self.nw::<1>(&next, -alpha, d, ply + 1, timer)? <= alpha {
                            #[cfg(not(test))]
                            // The late move pruning heuristic is not exact.
                            return Ok(None);
                        }
                    }
                }

                let pv = match -self.nw(&next, -alpha, depth, ply + 1, timer)? {
                    pv if pv <= alpha || pv >= beta => pv,
                    pv => match Score::new(cutoff.fetch_max(pv.score().get(), Ordering::Relaxed)) {
                        a if a >= beta => return Ok(None),
                        a => {
                            let alpha = pv.score().max(a);
                            pv.max(-self.ns(&next, -beta..-alpha, depth, ply + 1, timer)?)
                        }
                    },
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
        let mut best = Pv::new(Score::new(0), []);

        'id: for d in 0..=depth.get() {
            let mut w: i16 = 32;
            let mut lower = (best.score() - w / 2).min(Score::UPPER - w);
            let mut upper = (best.score() + w / 2).max(Score::LOWER + w);

            best = 'aw: loop {
                let timer = if best.is_empty() {
                    // Ignore time limits until some pv is found.
                    Timer::disarmed()
                } else {
                    timer
                };

                let depth = Depth::new(d);
                let pos = Evaluator::borrow(pos);
                let pv = match self.ns(&pos, lower..upper, depth, Ply::new(0), timer) {
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
    use std::time::{Duration, Instant};
    use test_strategy::proptest;

    fn negamax(pos: &Evaluator, depth: Depth, ply: Ply) -> Score {
        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Score::new(0),
            Some(_) => return Score::LOWER.normalize(ply),
            None => pos.value().cast(),
        };

        let kind = if ply >= Ply::UPPER {
            return score;
        } else if ply >= depth && !pos.is_check() {
            MoveKind::CAPTURE | MoveKind::PROMOTION
        } else {
            MoveKind::ANY
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
        let e = Engine::with_options(o);
        prop_assume!(e.tt.capacity() > 1);
        assert!(e.tt.size() <= o.hash);
    }

    #[proptest]
    #[should_panic]
    fn nw_panics_if_beta_is_too_small(e: Engine, pos: Position, d: Depth, p: Ply) {
        let pos = Evaluator::borrow(&pos);
        e.nw::<1>(&pos, Score::LOWER, d, p, Timer::disarmed())?;
    }

    #[proptest]
    fn nw_returns_transposition_if_beta_too_low(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[filter(#b >= Value::LOWER)] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#s.mate().is_none() && #s >= #b)] s: Score,
        selector: Selector,
    ) {
        let m = *selector.select(pos.moves(MoveKind::ANY));
        e.tt.set(pos.zobrist(), Transposition::lower(d, s, m));

        assert_eq!(
            e.nw(&Evaluator::borrow(&pos), b, d, p, Timer::disarmed()),
            Ok(Pv::<1>::new(s, [m]))
        );
    }

    #[proptest]
    fn nw_returns_transposition_if_beta_too_high(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[filter(#b >= Value::LOWER)] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#s.mate().is_none() && #s <= #b)] s: Score,
        selector: Selector,
    ) {
        let m = *selector.select(pos.moves(MoveKind::ANY));
        e.tt.set(pos.zobrist(), Transposition::upper(d, s, m));

        assert_eq!(
            e.nw(&Evaluator::borrow(&pos), b, d, p, Timer::disarmed()),
            Ok(Pv::<1>::new(s, [m]))
        );
    }

    #[proptest]
    fn nw_returns_transposition_if_exact(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[filter(#b >= Value::LOWER)] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#sc.mate().is_none())] sc: Score,
        selector: Selector,
    ) {
        let m = *selector.select(pos.moves(MoveKind::ANY));
        e.tt.set(pos.zobrist(), Transposition::exact(d, sc, m));

        assert_eq!(
            e.nw(&Evaluator::borrow(&pos), b, d, p, Timer::disarmed()),
            Ok(Pv::<1>::new(sc, [m]))
        );
    }

    #[proptest]
    #[should_panic]
    fn ns_panics_if_alpha_is_not_greater_than_beta(
        e: Engine,
        pos: Position,
        b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        e.ns::<1>(&pos, b.end..b.start, d, p, Timer::disarmed())?;
    }

    #[proptest]
    fn ns_aborts_if_time_is_up(
        e: Engine,
        pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::start(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(e.ns::<1>(&pos, b, d, p, timer), Err(Timeout));
    }

    #[proptest]
    fn ns_returns_drawn_score_if_game_ends_in_a_draw(
        e: Engine,
        #[filter(#pos.outcome().is_some_and(|o| o.is_draw()))] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        assert_eq!(
            e.ns(&Evaluator::borrow(&pos), b, d, p, Timer::disarmed()),
            Ok(Pv::<1>::new(Score::new(0), []))
        );
    }

    #[proptest]
    fn ns_returns_lost_score_if_game_ends_in_checkmate(
        e: Engine,
        #[filter(#pos.outcome().is_some_and(|o| o.is_decisive()))] pos: Position,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        assert_eq!(
            e.ns(&Evaluator::borrow(&pos), b, d, p, Timer::disarmed()),
            Ok(Pv::<1>::new(Score::LOWER.normalize(p), []))
        );
    }

    #[proptest]
    fn ns_finds_best_score(e: Engine, pos: Position, d: Depth, #[filter(#p >= 0)] p: Ply) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::disarmed();
        let bounds = Score::LOWER..Score::UPPER;

        assert_eq!(e.ns::<1>(&pos, bounds, d, p, timer)?, negamax(&pos, d, p));
    }

    #[proptest]
    fn ns_does_not_depend_on_configuration(
        x: Options,
        y: Options,
        pos: Position,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let x = Engine::with_options(x);
        let y = Engine::with_options(y);

        let pos = Evaluator::borrow(&pos);
        let bounds = Score::LOWER..Score::UPPER;
        let timer = Timer::disarmed();

        assert_eq!(
            x.ns::<1>(&pos, bounds.clone(), d, p, timer)?.score(),
            y.ns::<1>(&pos, bounds, d, p, timer)?.score()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(mut e: Engine, pos: Position, d: Depth) {
        let pos = Evaluator::borrow(&pos);
        let timer = Timer::disarmed();
        let bounds = Score::LOWER..Score::UPPER;

        assert_eq!(
            e.search::<3>(&pos, Limits::Depth(d)).score(),
            e.ns::<3>(&pos, bounds, d, Ply::new(0), timer)?.score()
        );
    }

    #[proptest]
    fn search_is_stable(mut e: Engine, pos: Position, d: Depth) {
        assert_eq!(
            e.search::<3>(&pos, Limits::Depth(d)).score(),
            e.search::<3>(&pos, Limits::Depth(d)).score()
        );
    }

    #[proptest]
    fn search_can_be_limited_by_time(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[strategy(..10u8)] ms: u8,
    ) {
        let t = Instant::now();
        e.search::<1>(&pos, Limits::Time(Duration::from_millis(ms.into())));
        assert!(t.elapsed() < Duration::from_secs(1));
    }

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
    ) {
        assert!(!e.search::<1>(&pos, Limits::Time(Duration::ZERO)).is_empty());
    }
}
