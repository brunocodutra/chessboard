use crate::chess::{Move, Piece, Position, Role};
use crate::nnue::{Evaluator, Value};
use crate::search::{Depth, DepthBounds, Killers, Limits, Options, Ply, Pv, Score};
use crate::search::{Transposition, TranspositionTable};
use crate::util::{Assume, Bounds, Buffer, Counter, Timer};
use derive_more::{Display, Error};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::sync::atomic::{AtomicI16, Ordering};
use std::{cell::RefCell, cmp::max, ops::Range, time::Duration};

#[cfg(test)]
use crate::search::{HashSize, ThreadCount};

/// Indicates the search was interrupted upon reaching the configured limit.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display("the search was interrupted")]
struct Interrupted;

/// The search control.
#[derive(Debug)]
struct Control(Counter, Timer);

impl Default for Control {
    fn default() -> Self {
        Control(Counter::new(u64::MAX), Timer::new(Duration::MAX))
    }
}

/// A chess engine.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Engine {
    #[cfg_attr(test, map(|c: ThreadCount| ThreadPoolBuilder::new().num_threads(c.get()).build().unwrap()))]
    executor: ThreadPool,
    #[cfg_attr(test, map(|s: HashSize| TranspositionTable::new(s)))]
    tt: TranspositionTable,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    thread_local! {
        static KILLERS: RefCell<Killers<2, { DepthBounds::UPPER as _ }>> = const {
            RefCell::new(Killers::new())
        };
    }

    /// Initializes the engine with the default [`Options`].
    pub fn new() -> Self {
        Self::with_options(Options::default())
    }

    /// Initializes the engine with the given [`Options`].
    pub fn with_options(options: Options) -> Self {
        Engine {
            tt: TranspositionTable::new(options.hash),
            executor: ThreadPoolBuilder::new()
                .num_threads(options.threads.get())
                .build()
                .expect("failed to initialize thread pool"),
        }
    }

    /// Records a `[Transposition`].
    fn record(
        &self,
        pos: &Position,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        score: Score,
        best: Move,
    ) {
        self.tt.set(
            pos.zobrist(),
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
    fn nmp(
        &self,
        pos: &Position,
        guess: Score,
        beta: Score,
        depth: Depth,
        ply: Ply,
    ) -> Option<Depth> {
        let turn = pos.turn();
        let pawn = Piece::new(Role::Pawn, turn);
        if guess > beta && pos.by_color(turn).len() - pos.by_piece(pawn).len() > 1 {
            Some(depth - 2 - (depth - ply) / 4)
        } else {
            None
        }
    }

    /// An implementation of [late move pruning].
    ///
    /// [late move pruning]: https://www.chessprogramming.org/Late_Move_Reductions
    fn lmp(
        &self,
        next: &Evaluator,
        m: Move,
        alpha: Value,
        depth: Depth,
        ply: Ply,
    ) -> Option<Depth> {
        let bounds = -(alpha - 60)..-(alpha - 501);
        let r = match (alpha + next.clone().see(m.whither(), bounds)).get() {
            ..=60 => return None,
            61..=180 => 1,
            181..=500 => 2,
            _ => 3,
        };

        Some(depth - r - (depth - ply) / 4)
    }

    /// A [zero-window] alpha-beta search.
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw(
        &self,
        pos: &Evaluator,
        beta: Score,
        depth: Depth,
        ply: Ply,
        ctrl: &Control,
    ) -> Result<Pv, Interrupted> {
        self.pvs(pos, beta - 1..beta, depth, ply, ctrl)
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
        ctrl @ Control(nodes, timer): &Control,
    ) -> Result<Pv, Interrupted> {
        assert!(!bounds.is_empty(), "{bounds:?} ≠ ∅");

        nodes.count().ok_or(Interrupted)?;
        timer.remaining().ok_or(Interrupted)?;

        let in_check = match pos.outcome() {
            Some(o) if o.is_draw() => return Ok(Pv::new(Score::new(0), [])),
            Some(_) => return Ok(Pv::new(Score::LOWER.normalize(ply), [])),
            None => pos.is_check(),
        };

        let (alpha, beta) = self.mdp(ply, &bounds);
        let tpos = self.tt.get(pos.zobrist());
        let is_pv = alpha + 1 < beta;

        let score = match tpos {
            Some(t) => t.score().normalize(ply),
            _ => pos.evaluate().cast(),
        };

        let depth = match tpos {
            #[cfg(not(test))]
            // Reductions are not exact.
            None => depth - 1,
            _ => depth,
        };

        let quiesce = ply >= depth && !in_check;
        let alpha = match quiesce {
            #[cfg(not(test))]
            // The stand pat heuristic is not exact.
            true => alpha.max(score),
            _ => alpha,
        };

        if alpha >= beta {
            return Ok(Pv::new(alpha, []));
        } else if let Some(t) = tpos {
            if !is_pv && t.depth() >= depth - ply {
                let (lower, upper) = t.bounds().into_inner();
                if lower == upper || upper <= alpha || lower >= beta {
                    return Ok(Pv::new(score, [t.best()]));
                }
            }
        }

        if ply >= Ply::UPPER {
            return Ok(Pv::new(score, []));
        } else if !is_pv && !in_check {
            if let Some(d) = self.nmp(pos, score, beta, depth, ply) {
                let mut next = pos.clone();
                next.pass();
                if d <= ply || -self.nw(&next, -beta + 1, d, ply + 1, ctrl)? >= beta {
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Pv::new(score, []));
                }
            }
        }

        let mut moves = Buffer::<_, 256>::from_iter(pos.moves().filter_map(|m| {
            if quiesce && m.is_quiet() {
                None
            } else if Some(m) == tpos.map(|t| t.best()) {
                Some((m, Value::UPPER))
            } else if Self::KILLERS.with_borrow(|ks| ks.contains(ply, pos.turn(), m)) {
                Some((m, Value::new(100)))
            } else if m.is_quiet() {
                Some((m, Value::new(0)))
            } else {
                let mut next = pos.material();
                let material = next.evaluate();
                next.play(m);
                Some((m, -next.evaluate() - material))
            }
        }));

        moves.sort_unstable_by_key(|(_, gain)| *gain);

        let best = match moves.pop() {
            None => return Ok(Pv::new(score, [])),
            Some((m, _)) => {
                let mut next = pos.clone();
                next.play(m);
                let mut pv = -self.pvs(&next, -beta..-alpha, depth, ply + 1, ctrl)?;
                pv.shift(m);

                if pv >= beta && m.is_quiet() {
                    Self::KILLERS.with_borrow_mut(|ks| ks.insert(ply, pos.turn(), m));
                }

                pv
            }
        };

        if best >= beta {
            self.record(pos, bounds, depth, ply, best.score(), best[0]);
            return Ok(best);
        }

        let cutoff = AtomicI16::new(best.score().max(alpha).get());

        let (best, _) = moves
            .into_par_iter()
            .rev()
            .map(|&(m, gain)| {
                let alpha = Score::new(cutoff.load(Ordering::Relaxed));

                if alpha >= beta {
                    return Ok(None);
                }

                let mut next = pos.clone();
                next.play(m);

                if gain < 100 && !in_check && !next.is_check() {
                    if let Some(d) = self.lmp(&next, m, alpha.cast(), depth, ply) {
                        if d <= ply || -self.nw(&next, -alpha, d, ply + 1, ctrl)? <= alpha {
                            #[cfg(not(test))]
                            // The late move pruning heuristic is not exact.
                            return Ok(None);
                        }
                    }
                }

                let mut pv = match -self.nw(&next, -alpha, depth, ply + 1, ctrl)? {
                    pv if pv <= alpha || pv >= beta => pv,
                    _ => -self.pvs(&next, -beta..-alpha, depth, ply + 1, ctrl)?,
                };

                if pv >= beta && m.is_quiet() {
                    Self::KILLERS.with_borrow_mut(|ks| ks.insert(ply, pos.turn(), m));
                }

                if pv > alpha {
                    cutoff.fetch_max(pv.score().get(), Ordering::Relaxed);
                }

                pv.shift(m);
                Ok(Some((pv, gain)))
            })
            .chain([Ok(Some((best, Value::UPPER)))])
            .try_reduce(|| None, |a, b| Ok(max(a, b)))?
            .assume();

        self.record(pos, bounds, depth, ply, best.score(), best[0]);

        Ok(best)
    }

    /// An implementation of [aspiration windows] with [iterative deepening].
    ///
    /// [aspiration windows]: https://www.chessprogramming.org/Aspiration_Windows
    /// [iterative deepening]: https://www.chessprogramming.org/Iterative_Deepening
    fn aw(&self, pos: &Evaluator, depth: Depth, nodes: u64, time: Range<Duration>) -> Pv {
        let ref ctrl @ Control(_, ref timer) = Control(Counter::new(nodes), Timer::new(time.end));
        let mut best = Pv::new(Score::new(0), []);

        for d in 0..=1 {
            let depth = Depth::new(d);
            let bounds = Score::LOWER..Score::UPPER;
            let ctrl = Control::default();
            best = self.pvs(pos, bounds, depth, Ply::new(0), &ctrl).assume();
        }

        'id: for d in 2..=depth.get() {
            if timer.remaining() < Duration::checked_sub(time.end, time.start) {
                break 'id;
            }

            let mut w: i16 = 32;
            let mut lower = (best.score() - w / 2).min(Score::UPPER - w);
            let mut upper = (best.score() + w / 2).max(Score::LOWER + w);

            best = 'aw: loop {
                let depth = Depth::new(d);
                let pv = match self.pvs(pos, lower..upper, depth, Ply::new(0), ctrl) {
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

    fn time_to_search(&self, pos: &Position, limits: Limits) -> Range<Duration> {
        let (clock, inc) = match limits {
            Limits::Clock(c, i) => (c, i),
            _ => return limits.time()..limits.time(),
        };

        let cap = clock.mul_f32(0.8);
        let excess = clock.saturating_sub(inc);
        let scale = 400 / pos.fullmoves().get().min(40);
        let max = inc.saturating_add(excess / scale).min(cap);
        max / 2..max
    }

    /// Searches for the [principal variation][`Pv`].
    pub fn search(&mut self, pos: &Position, limits: Limits) -> Pv {
        let depth = limits.depth();
        let nodes = limits.nodes();
        let time = self.time_to_search(pos, limits);
        let pos = Evaluator::new(pos.clone());
        self.executor.install(|| self.aw(&pos, depth, nodes, time))
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
            None => pos.evaluate().cast(),
        };

        if ply >= Ply::UPPER {
            return score;
        }

        pos.moves()
            .filter(|m| ply < depth || pos.is_check() || !m.is_quiet())
            .map(|m| {
                let mut next = pos.clone();
                next.play(m);
                -negamax(&next, depth, ply + 1)
            })
            .max()
            .unwrap_or(score)
    }

    #[proptest]
    fn hash_is_an_upper_limit_for_table_size(o: Options) {
        let e = Engine::with_options(o);
        prop_assume!(e.tt.capacity() > 1);
        assert!(e.tt.size() <= o.hash);
    }

    #[proptest]
    #[should_panic]
    fn nw_panics_if_beta_is_too_small(e: Engine, pos: Evaluator, d: Depth, p: Ply) {
        e.nw(&pos, Score::LOWER, d, p, &Control::default())?;
    }

    #[proptest]
    fn nw_returns_transposition_if_beta_too_low(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter((Value::LOWER..Value::UPPER).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#s.mate().is_none() && #s >= #b)] s: Score,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        e.tt.set(pos.zobrist(), Transposition::lower(d, s, m));

        let ctrl = Control::default();
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(s, [m])));
    }

    #[proptest]
    fn nw_returns_transposition_if_beta_too_high(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter((Value::LOWER..Value::UPPER).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#s.mate().is_none() && #s < #b)] s: Score,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        e.tt.set(pos.zobrist(), Transposition::upper(d, s, m));

        let ctrl = Control::default();
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(s, [m])));
    }

    #[proptest]
    fn nw_returns_transposition_if_exact(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter((Value::LOWER..Value::UPPER).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#sc.mate().is_none())] sc: Score,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        e.tt.set(pos.zobrist(), Transposition::exact(d, sc, m));

        let ctrl = Control::default();
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(sc, [m])));
    }

    #[proptest]
    #[should_panic]
    fn pvs_panics_if_alpha_is_not_greater_than_beta(
        e: Engine,
        pos: Evaluator,
        b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        e.pvs(&pos, b.end..b.start, d, p, &Control::default())?;
    }

    #[proptest]
    fn pvs_aborts_if_maximum_number_of_nodes_visited(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let ctrl = Control(Counter::new(0), Timer::new(Duration::MAX));
        assert_eq!(e.pvs(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let ctrl = Control(Counter::new(u64::MAX), Timer::new(Duration::ZERO));
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(e.pvs(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn pvs_returns_static_evaluation_if_max_ply(
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
    ) {
        assert_eq!(
            e.pvs(&pos, b, d, Ply::UPPER, &Control::default()),
            Ok(Pv::new(pos.evaluate().cast(), []))
        );
    }

    #[proptest]
    fn pvs_returns_drawn_score_if_game_ends_in_a_draw(
        e: Engine,
        #[filter(#pos.outcome().is_some_and(|o| o.is_draw()))] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let ctrl = Control::default();
        assert_eq!(e.pvs(&pos, b, d, p, &ctrl), Ok(Pv::new(Score::new(0), [])));
    }

    #[proptest]
    fn pvs_returns_lost_score_if_game_ends_in_checkmate(
        e: Engine,
        #[filter(#pos.outcome().is_some_and(|o| o.is_decisive()))] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        assert_eq!(
            e.pvs(&pos, b, d, p, &Control::default()),
            Ok(Pv::new(Score::LOWER.normalize(p), []))
        );
    }

    #[proptest]
    fn pvs_finds_best_score(e: Engine, pos: Evaluator, d: Depth, #[filter(#p >= 0)] p: Ply) {
        let ctrl = Control::default();
        let bounds = Score::LOWER..Score::UPPER;
        assert_eq!(e.pvs(&pos, bounds, d, p, &ctrl)?, negamax(&pos, d, p));
    }

    #[proptest]
    fn pvs_does_not_depend_on_configuration(
        x: Options,
        y: Options,
        pos: Evaluator,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let x = Engine::with_options(x);
        let y = Engine::with_options(y);

        let bounds = Score::LOWER..Score::UPPER;
        let ctrl = Control::default();

        assert_eq!(
            x.pvs(&pos, bounds.clone(), d, p, &ctrl)?.score(),
            y.pvs(&pos, bounds, d, p, &ctrl)?.score()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(
        mut e: Engine,
        pos: Position,
        #[filter(#d > 1)] d: Depth,
    ) {
        let ctrl = Control::default();
        let bounds = Score::LOWER..Score::UPPER;
        let ply = Ply::new(0);

        assert_eq!(
            e.search(&pos, Limits::Depth(d)).score(),
            e.pvs(&Evaluator::new(pos), bounds, d, ply, &ctrl)?.score()
        );
    }

    #[proptest]
    fn search_is_stable(mut e: Engine, pos: Position, d: Depth) {
        assert_eq!(
            e.search(&pos, Limits::Depth(d)).score(),
            e.search(&pos, Limits::Depth(d)).score()
        );
    }

    #[proptest]
    fn search_can_be_limited_by_time(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[strategy(..10u8)] ms: u8,
    ) {
        let t = Instant::now();
        e.search(&pos, Limits::Time(Duration::from_millis(ms.into())));
        assert!(t.elapsed() < Duration::from_secs(1));
    }

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Position,
    ) {
        assert!(!e.search(&pos, Limits::Time(Duration::ZERO)).is_empty());
    }
}
