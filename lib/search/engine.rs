use crate::nnue::{Evaluator, Value};
use crate::util::{Assume, Counter, Integer, Timer, Trigger};
use crate::{chess::Move, search::*};
use arrayvec::ArrayVec;
use std::{cell::RefCell, ops::Range, time::Duration};

#[cfg(test)]
use crate::search::{HashSize, ThreadCount};

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
        let upper = Score::upper().normalize(ply + 1); // One can't mate in 0 plies!
        (bounds.start.max(lower), bounds.end.min(upper))
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
        if guess >= beta && pos.pieces(turn).len() > 1 {
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
        let bounds = -(alpha - 24)..-(alpha - 161);
        let r = match (alpha + next.clone().see(m.whither(), bounds)).get() {
            ..=24 => return None,
            25..=80 => 1,
            81..=160 => 2,
            _ => 3,
        };

        Some(depth - r - (depth - ply) / 4)
    }

    /// A full alpha-beta search.
    fn fw(
        &self,
        pos: &Evaluator,
        depth: Depth,
        ply: Ply,
        ctrl: &Control,
    ) -> Result<Pv, Interrupted> {
        self.pvs::<true>(pos, Score::lower()..Score::upper(), depth, ply, ctrl)
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
        self.pvs::<false>(pos, beta - 1..beta, depth, ply, ctrl)
    }

    /// An implementation of the [PVS] variation of [alpha-beta pruning] algorithm.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    /// [alpha-beta pruning]: https://www.chessprogramming.org/Alpha-Beta
    fn pvs<const PV: bool>(
        &self,
        pos: &Evaluator,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        ctrl: &Control,
    ) -> Result<Pv, Interrupted> {
        debug_assert!(!bounds.is_empty());

        ctrl.interrupted()?;

        let (alpha, beta) = self.mdp(ply, &bounds);
        if alpha >= beta {
            return Ok(Pv::new(alpha, None));
        }

        let transposition = match pos.outcome() {
            Some(o) if o.is_draw() => return Ok(Pv::new(Score::new(0), None)),
            Some(_) => return Ok(Pv::new(Score::lower().normalize(ply), None)),
            None => self.tt.get(pos.zobrist()),
        };

        let depth = match transposition {
            #[cfg(not(test))]
            // Extensions are not exact.
            Some(_) if pos.is_check() => depth + 1,

            #[cfg(not(test))]
            // Reductions are not exact.
            None if !pos.is_check() => depth - 1,

            _ => depth,
        };

        if let Some(t) = transposition {
            if !PV && t.depth() >= depth - ply {
                let (lower, upper) = t.bounds().into_inner();
                if lower >= upper || upper <= alpha || lower >= beta {
                    return Ok(Pv::new(t.score().normalize(ply), Some(t.best())));
                }
            }
        }

        let score = match transposition {
            Some(t) => t.score().normalize(ply),
            _ => pos.evaluate().saturate(),
        };

        let quiesce = ply >= depth;
        let alpha = match quiesce {
            #[cfg(not(test))]
            // The stand pat heuristic is not exact.
            true => alpha.max(score),
            _ => alpha,
        };

        if alpha >= beta || ply >= Ply::MAX {
            return Ok(Pv::new(score, None));
        } else if !PV && !pos.is_check() {
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
                if Some(m) == transposition.map(|t| t.best()) {
                    (m, Value::upper())
                } else if Self::KILLERS.with_borrow(|ks| ks.contains(ply, pos.turn(), m)) {
                    (m, Value::new(40))
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
                m >> -self.pvs::<PV>(&next, -beta..-alpha, depth, ply + 1, ctrl)?
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

            self.tt.prefetch(next.zobrist());
            if gain <= 0 && !pos.is_check() && !next.is_check() {
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
                _ => m >> -self.pvs::<PV>(&next, -beta..-alpha, depth, ply + 1, ctrl)?,
            };

            Ok(pv)
        })?;

        Ok(self.record(pos, bounds, depth, ply, pv))
    }

    /// An implementation of [aspiration windows] with [iterative deepening].
    ///
    /// [aspiration windows]: https://www.chessprogramming.org/Aspiration_Windows
    /// [iterative deepening]: https://www.chessprogramming.org/Iterative_Deepening
    fn aw(
        &self,
        pos: &Evaluator,
        limit: Depth,
        nodes: u64,
        time: Range<Duration>,
        interrupter: &Trigger,
    ) -> Pv {
        let ctrl = Control::Limited(Counter::new(nodes), Timer::new(time.end), interrupter);
        let mut pv = Pv::new(Score::new(0), None);
        let mut depth = Depth::new(0);

        while depth < Depth::upper() {
            use Control::*;
            pv = self.fw(pos, depth, Ply::new(0), &Unlimited).assume();
            depth = depth + 1;
            if pv.best().is_some() {
                break;
            }
        }

        'id: for d in depth.get()..=limit.get() {
            let mut overtime = time.end - time.start;
            let mut depth = Depth::new(d);
            let mut delta: i16 = 10;

            let (mut lower, mut upper) = match d {
                ..=4 => (Score::lower(), Score::upper()),
                _ => (pv.score() - delta, pv.score() + delta),
            };

            pv = 'aw: loop {
                delta = delta.saturating_mul(2);
                if ctrl.timer().remaining() < Some(overtime) {
                    break 'id;
                }

                let bounds = lower..upper;
                let Ok(partial) = self.pvs::<true>(pos, bounds, depth, Ply::new(0), &ctrl) else {
                    break 'id;
                };

                match partial.score() {
                    score if (-lower..Score::upper()).contains(&-score) => {
                        overtime /= 2;
                        depth = Depth::new(d);
                        upper = lower / 2 + upper / 2;
                        lower = score - delta;
                    }

                    score if (upper..Score::upper()).contains(&score) => {
                        overtime = time.end - time.start;
                        depth = depth - 1;
                        upper = score + delta;
                        pv = partial;
                    }

                    _ => break 'aw partial,
                }
            };
        }

        pv
    }

    fn time_to_search(&self, pos: &Evaluator, limits: Limits) -> Range<Duration> {
        let (clock, inc) = match limits {
            Limits::Clock(c, i) => (c, i),
            _ => return limits.time()..limits.time(),
        };

        let time_left = clock.saturating_sub(inc);
        let moves_left = 280 / pos.fullmoves().get().min(40);
        let time_per_move = inc.saturating_add(time_left / moves_left);
        time_per_move / 2..time_per_move
    }

    /// Searches for the [principal variation][`Pv`].
    pub fn search(&mut self, pos: &Evaluator, limits: Limits, interrupter: &Trigger) -> Pv {
        let time = self.time_to_search(pos, limits);
        let (depth, nodes) = (limits.depth(), limits.nodes());
        self.aw(pos, depth, nodes, time, interrupter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assume, sample::Selector};
    use std::time::Instant;
    use test_strategy::proptest;

    fn alphabeta(pos: &Evaluator, bounds: Range<Score>, depth: Depth, ply: Ply) -> Score {
        let (mut alpha, beta) = (bounds.start, bounds.end);
        debug_assert!(alpha < beta);

        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Score::new(0),
            Some(_) => return Score::lower().normalize(ply),
            None => pos.evaluate().saturate(),
        };

        let moves: ArrayVec<_, 255> = pos
            .moves()
            .filter(|m| ply < depth || !m.is_quiet())
            .flatten()
            .collect();

        if ply >= Ply::MAX || moves.is_empty() {
            return score;
        }

        for m in moves {
            let mut next = pos.clone();
            next.play(m);
            let score = -alphabeta(&next, -beta..-alpha, depth, ply + 1);
            alpha = score.max(alpha);
            if alpha >= beta {
                break;
            }
        }

        alpha
    }

    fn negamax(pos: &Evaluator, depth: Depth, ply: Ply) -> Score {
        alphabeta(pos, Score::lower()..Score::upper(), depth, ply)
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
        e.nw(&pos, Score::lower(), d, p, &Control::Unlimited)?;
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

        let ctrl = Control::Unlimited;
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

        let ctrl = Control::Unlimited;
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

        let ctrl = Control::Unlimited;
        assert_eq!(e.nw(&pos, b, d, p, &ctrl), Ok(Pv::new(sc, Some(m))));
    }

    #[proptest]
    fn nw_finds_score_bound(
        e: Engine,
        pos: Evaluator,
        #[filter((Value::lower()..Value::upper()).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        assert_eq!(
            e.nw(&pos, b, d, p, &Control::Unlimited)? < b,
            alphabeta(&pos, b - 1..b, d, p) < b
        );
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
        e.pvs::<true>(&pos, b.end..b.start, d, p, &Control::Unlimited)?;
    }

    #[proptest]
    fn pvs_aborts_if_maximum_number_of_nodes_visited(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let interrupter = Trigger::armed();
        let ctrl = Control::Limited(Counter::new(0), Timer::infinite(), &interrupter);
        assert_eq!(e.pvs::<true>(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn pvs_aborts_if_time_is_up(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let interrupter = Trigger::armed();
        let ctrl = Control::Limited(
            Counter::new(u64::MAX),
            Timer::new(Duration::ZERO),
            &interrupter,
        );
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(e.pvs::<true>(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn pvs_aborts_if_interrupter_is_disarmed(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        p: Ply,
    ) {
        let interrupter = Trigger::disarmed();
        let ctrl = Control::Limited(Counter::new(u64::MAX), Timer::infinite(), &interrupter);
        assert_eq!(e.pvs::<true>(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn pvs_returns_static_evaluation_if_max_ply(
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
    ) {
        assert_eq!(
            e.pvs::<true>(&pos, b, d, Ply::upper(), &Control::Unlimited),
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
        assert_eq!(
            e.pvs::<true>(&pos, b, d, p, &Control::Unlimited),
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
            e.pvs::<true>(&pos, b, d, p, &Control::Unlimited),
            Ok(Pv::new(Score::lower().normalize(p), None))
        );
    }

    #[proptest]
    fn fw_finds_best_score(e: Engine, pos: Evaluator, d: Depth, #[filter(#p >= 0)] p: Ply) {
        assert_eq!(e.fw(&pos, d, p, &Control::Unlimited)?, negamax(&pos, d, p));
    }

    #[proptest]
    fn fw_does_not_depend_on_configuration(
        x: Options,
        y: Options,
        pos: Evaluator,
        d: Depth,
        #[filter(#p >= 0)] p: Ply,
    ) {
        let x = Engine::with_options(x);
        let y = Engine::with_options(y);

        let ctrl = Control::Unlimited;

        assert_eq!(
            x.fw(&pos, d, p, &ctrl)?.score(),
            y.fw(&pos, d, p, &ctrl)?.score()
        );
    }

    #[proptest]
    fn search_finds_the_principal_variation(
        mut e: Engine,
        pos: Evaluator,
        #[filter(#d > 1)] d: Depth,
    ) {
        assert_eq!(
            e.search(&pos, Limits::Depth(d), &Trigger::armed()).score(),
            e.fw(&pos, d, Ply::new(0), &Control::Unlimited)?.score()
        );
    }

    #[proptest]
    fn search_is_stable(mut e: Engine, pos: Evaluator, d: Depth) {
        assert_eq!(
            e.search(&pos, Limits::Depth(d), &Trigger::armed()).score(),
            e.search(&pos, Limits::Depth(d), &Trigger::armed()).score()
        );
    }

    #[proptest]
    fn search_can_be_limited_by_time(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[strategy(..10u8)] ms: u8,
    ) {
        let timer = Instant::now();
        let trigger = Trigger::armed();
        let limits = Limits::Time(Duration::from_millis(ms.into()));
        e.search(&pos, limits, &trigger);
        assert!(timer.elapsed() < Duration::from_secs(1));
    }

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        let limits = Duration::ZERO.into();
        assert_ne!(e.search(&pos, limits, &Trigger::armed()).best(), None);
    }

    #[proptest]
    fn search_extends_depth_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        let limits = Depth::lower().into();
        assert_ne!(e.search(&pos, limits, &Trigger::armed()).best(), None);
    }

    #[proptest]
    fn search_ignores_interrupter_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        let limits = Limits::None;
        assert_ne!(e.search(&pos, limits, &Trigger::disarmed()).best(), None);
    }
}
