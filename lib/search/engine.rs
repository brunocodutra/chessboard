use crate::nnue::{Evaluator, Value};
use crate::util::{Assume, Counter, Integer, Timer};
use crate::{chess::Move, search::*};
use arrayvec::ArrayVec;
use std::{cell::RefCell, ops::Range, time::Duration};

#[cfg(test)]
use crate::search::{HashSize, ThreadCount};

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
    #[cfg_attr(test, map(|c: ThreadCount| Driver::new(c)))]
    driver: Driver,
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
        static KILLERS: RefCell<Killers<{ Depth::MAX as _ }>> = const {
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
            driver: Driver::new(options.threads),
            tt: TranspositionTable::new(options.hash),
        }
    }

    /// Records a `[Transposition`].
    fn record(&self, pos: &Evaluator, bounds: Range<Score>, depth: Depth, ply: Ply, pv: Pv) -> Pv {
        let m = pv.best().assume();
        if pv >= bounds.end && m.is_quiet() {
            Self::KILLERS.with_borrow_mut(|ks| ks.insert(ply, pos.turn(), m));
        }

        self.tt.set(
            pos.zobrist(),
            if pv.score() >= bounds.end {
                Transposition::lower(depth - ply, pv.score().normalize(-ply), m)
            } else if pv.score() <= bounds.start {
                Transposition::upper(depth - ply, pv.score().normalize(-ply), m)
            } else {
                Transposition::exact(depth - ply, pv.score().normalize(-ply), m)
            },
        );

        pv
    }

    /// An implementation of [mate distance pruning].
    ///
    /// [mate distance pruning]: https://www.chessprogramming.org/Mate_Distance_Pruning
    fn mdp(&self, ply: Ply, bounds: &Range<Score>) -> (Score, Score) {
        let lower = Score::lower().normalize(ply);
        let upper = (Score::upper() - 1).normalize(ply); // One can't mate in 0 plies!
        let alpha = bounds.start.clamp(lower, upper);
        let beta = bounds.end.clamp(lower, upper);
        (alpha, beta)
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(
        &self,
        pos: &Evaluator,
        guess: Score,
        beta: Score,
        depth: Depth,
        ply: Ply,
    ) -> Option<Depth> {
        let turn = pos.turn();
        if guess > beta && pos.pieces(turn).len() > 1 {
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
            Some(o) if o.is_draw() => return Ok(Pv::new(Score::new(0), None)),
            Some(_) => return Ok(Pv::new(Score::lower().normalize(ply), None)),
            None => pos.is_check(),
        };

        let (alpha, beta) = self.mdp(ply, &bounds);
        let tpos = self.tt.get(pos.zobrist());
        let is_pv = alpha + 1 < beta;

        let score = match tpos {
            Some(t) => t.score().normalize(ply),
            _ => pos.evaluate().saturate(),
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
            return Ok(Pv::new(alpha, None));
        } else if let Some(t) = tpos {
            if !is_pv && t.depth() >= depth - ply {
                let (lower, upper) = t.bounds().into_inner();
                if lower == upper || upper <= alpha || lower >= beta {
                    return Ok(Pv::new(score, Some(t.best())));
                }
            }
        }

        if ply >= Ply::MAX {
            return Ok(Pv::new(score, None));
        } else if !is_pv && !in_check {
            if let Some(d) = self.nmp(pos, score, beta, depth, ply) {
                let mut next = pos.clone();
                next.pass();
                if d <= ply || -self.nw(&next, -beta + 1, d, ply + 1, ctrl)? >= beta {
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(Pv::new(score, None));
                }
            }
        }

        let mut moves: ArrayVec<_, 255> = pos
            .moves()
            .filter(|ms| !quiesce || !ms.is_quiet())
            .flatten()
            .map(|m| {
                if Some(m) == tpos.map(|t| t.best()) {
                    (m, Value::upper())
                } else if Self::KILLERS.with_borrow(|ks| ks.contains(ply, pos.turn(), m)) {
                    (m, Value::new(100))
                } else if m.is_quiet() {
                    (m, Value::new(0))
                } else {
                    let mut next = pos.material();
                    let material = next.evaluate();
                    next.play(m);
                    (m, -next.evaluate() - material)
                }
            })
            .collect();

        moves.sort_unstable_by_key(|(_, gain)| *gain);

        let pv = match moves.pop() {
            None => return Ok(Pv::new(score, None)),
            Some((m, _)) => {
                let mut next = pos.clone();
                next.play(m);
                m >> -self.pvs(&next, -beta..-alpha, depth, ply + 1, ctrl)?
            }
        };

        if pv >= beta || moves.is_empty() {
            return Ok(self.record(pos, bounds, depth, ply, pv));
        }

        let pv = self.driver.drive(pv, &moves, |&best, &(m, gain)| {
            let alpha = match best.score() {
                s if s >= beta => return Err(ControlFlow::Break),
                s => s.max(alpha),
            };

            let mut next = pos.clone();
            next.play(m);

            if gain < 100 && !in_check && !next.is_check() {
                if let Some(d) = self.lmp(&next, m, alpha.saturate(), depth, ply) {
                    if d <= ply || -self.nw(&next, -alpha, d, ply + 1, ctrl)? <= alpha {
                        #[cfg(not(test))]
                        // The late move pruning heuristic is not exact.
                        return Ok(best);
                    }
                }
            }

            let pv = match -self.nw(&next, -alpha, depth, ply + 1, ctrl)? {
                pv if pv <= alpha || pv >= beta => m >> pv,
                _ => m >> -self.pvs(&next, -beta..-alpha, depth, ply + 1, ctrl)?,
            };

            Ok(pv)
        })?;

        Ok(self.record(pos, bounds, depth, ply, pv))
    }

    /// An implementation of [aspiration windows] with [iterative deepening].
    ///
    /// [aspiration windows]: https://www.chessprogramming.org/Aspiration_Windows
    /// [iterative deepening]: https://www.chessprogramming.org/Iterative_Deepening
    fn aw(&self, pos: &Evaluator, limit: Depth, nodes: u64, time: Range<Duration>) -> Pv {
        let ref ctrl @ Control(_, ref timer) = Control(Counter::new(nodes), Timer::new(time.end));
        let mut pv = Pv::new(Score::new(0), None);
        let mut depth = Depth::new(0);

        while depth < Depth::upper() {
            let ctrl = Control::default();
            let bounds = Score::lower()..Score::upper();
            pv = self.pvs(pos, bounds, depth, Ply::new(0), &ctrl).assume();
            depth = depth + 1;
            if pv.best().is_some() {
                break;
            }
        }

        'id: for d in depth.get()..=limit.get() {
            if timer.remaining() < Duration::checked_sub(time.end, time.start) {
                break 'id;
            }

            let depth = Depth::new(d);
            let mut window = Score::new(32);
            let mut lower = (pv.score() - window / 2).min(Score::upper() - window);
            let mut upper = (pv.score() + window / 2).max(Score::lower() + window);

            pv = 'aw: loop {
                let Ok(partial) = self.pvs(pos, lower..upper, depth, Ply::new(0), ctrl) else {
                    break 'id;
                };

                window = window * 2;
                match partial.score() {
                    s if (-lower..Score::upper()).contains(&-s) => lower = s - window / 2,
                    s if (upper..Score::upper()).contains(&s) => upper = s + window / 2,
                    _ => break 'aw partial,
                }

                pv = pv.max(partial);
            };
        }

        pv
    }

    fn time_to_search(&self, pos: &Evaluator, limits: Limits) -> Range<Duration> {
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
    pub fn search(&mut self, pos: &Evaluator, limits: Limits) -> Pv {
        let time = self.time_to_search(pos, limits);
        let (depth, nodes) = (limits.depth(), limits.nodes());
        self.aw(pos, depth, nodes, time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assume, sample::Selector};
    use std::time::Instant;
    use test_strategy::proptest;

    fn negamax(pos: &Evaluator, depth: Depth, ply: Ply) -> Score {
        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Score::new(0),
            Some(_) => return Score::lower().normalize(ply),
            None => pos.evaluate().saturate(),
        };

        if ply >= Ply::MAX {
            return score;
        }

        pos.moves()
            .flatten()
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
        e.nw(&pos, Score::lower(), d, p, &Control::default())?;
    }

    #[proptest]
    fn nw_returns_transposition_if_beta_too_low(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter((Value::lower()..Value::upper()).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#s.mate().is_none() && #s >= #b)] s: Score,
        #[map(|s: Selector| s.select(#pos.moves().flatten()))] m: Move,
    ) {
        e.tt.set(pos.zobrist(), Transposition::lower(d, s, m));

        let ctrl = Control::default();
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(s, Some(m))));
    }

    #[proptest]
    fn nw_returns_transposition_if_beta_too_high(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter((Value::lower()..Value::upper()).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#s.mate().is_none() && #s < #b)] s: Score,
        #[map(|s: Selector| s.select(#pos.moves().flatten()))] m: Move,
    ) {
        e.tt.set(pos.zobrist(), Transposition::upper(d, s, m));

        let ctrl = Control::default();
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(s, Some(m))));
    }

    #[proptest]
    fn nw_returns_transposition_if_exact(
        #[by_ref]
        #[filter(#e.tt.capacity() > 0)]
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter((Value::lower()..Value::upper()).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
        #[filter(#sc.mate().is_none())] sc: Score,
        #[map(|s: Selector| s.select(#pos.moves().flatten()))] m: Move,
    ) {
        e.tt.set(pos.zobrist(), Transposition::exact(d, sc, m));

        let ctrl = Control::default();
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(sc, Some(m))));
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
            e.pvs(&pos, b, d, Ply::upper(), &Control::default()),
            Ok(Pv::new(pos.evaluate().saturate(), None))
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
        assert_eq!(
            e.pvs(&pos, b, d, p, &ctrl),
            Ok(Pv::new(Score::new(0), None))
        );
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
            Ok(Pv::new(Score::lower().normalize(p), None))
        );
    }

    #[proptest]
    fn pvs_finds_best_score(e: Engine, pos: Evaluator, d: Depth, #[filter(#p >= 0)] p: Ply) {
        let ctrl = Control::default();
        let bounds = Score::lower()..Score::upper();
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

        let bounds = Score::lower()..Score::upper();
        let ctrl = Control::default();

        assert_eq!(
            x.pvs(&pos, bounds.clone(), d, p, &ctrl)?.score(),
            y.pvs(&pos, bounds, d, p, &ctrl)?.score()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(
        mut e: Engine,
        pos: Evaluator,
        #[filter(#d > 1)] d: Depth,
    ) {
        let ctrl = Control::default();
        let bounds = Score::lower()..Score::upper();
        let ply = Ply::new(0);

        assert_eq!(
            e.search(&pos, Limits::Depth(d)).score(),
            e.pvs(&pos, bounds, d, ply, &ctrl)?.score()
        );
    }

    #[proptest]
    fn search_is_stable(mut e: Engine, pos: Evaluator, d: Depth) {
        assert_eq!(
            e.search(&pos, Limits::Depth(d)).score(),
            e.search(&pos, Limits::Depth(d)).score()
        );
    }

    #[proptest]
    fn search_can_be_limited_by_time(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[strategy(..10u8)] ms: u8,
    ) {
        let t = Instant::now();
        e.search(&pos, Limits::Time(Duration::from_millis(ms.into())));
        assert!(t.elapsed() < Duration::from_secs(1));
    }

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        assert_ne!(e.search(&pos, Duration::ZERO.into()).best(), None);
    }

    #[proptest]
    fn search_extends_depth_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        assert_ne!(e.search(&pos, Depth::lower().into()).best(), None);
    }
}
