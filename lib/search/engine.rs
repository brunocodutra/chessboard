use crate::chess::{Move, Outcome};
use crate::nnue::{Evaluator, Value};
use crate::search::*;
use crate::util::{Assume, Counter, Integer, Timer, Trigger};
use arrayvec::ArrayVec;
use std::{ops::Range, time::Duration};

#[cfg(test)]
use crate::search::{HashSize, ThreadCount};

#[cfg(test)]
use proptest::strategy::LazyJust;

/// A chess engine.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Engine {
    #[cfg_attr(test, map(|c: ThreadCount| Driver::new(c)))]
    driver: Driver,
    #[cfg_attr(test, map(|s: HashSize| TranspositionTable::new(s)))]
    tt: TranspositionTable,
    #[cfg_attr(test, strategy(LazyJust::new(Killers::default)))]
    killers: Killers,
    #[cfg_attr(test, strategy(LazyJust::new(History::default)))]
    history: History,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    /// Initializes the engine with the default [`Options`].
    pub fn new() -> Self {
        Self::with_options(&Options::default())
    }

    /// Initializes the engine with the given [`Options`].
    pub fn with_options(options: &Options) -> Self {
        Engine {
            driver: Driver::new(options.threads),
            tt: TranspositionTable::new(options.hash),
            killers: Killers::default(),
            history: History::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        &self,
        pos: &Evaluator,
        moves: &[(Move, Value)],
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        best: Move,
        score: Score,
    ) {
        let draft = depth - ply;
        if score >= bounds.end && best.is_quiet() {
            self.killers.insert(ply, pos.turn(), best);
            self.history.update(best, pos.turn(), draft.get());
            for &(m, _) in moves.iter().rev() {
                if m == best {
                    break;
                } else if m.is_quiet() {
                    self.history.update(m, pos.turn(), -draft.get());
                }
            }
        }

        let score = ScoreBound::new(bounds, score, ply);
        let tpos = Transposition::new(score, draft, best);
        self.tt.set(pos.zobrist(), tpos);
    }

    /// An implementation of [mate distance pruning].
    ///
    /// [mate distance pruning]: https://www.chessprogramming.org/Mate_Distance_Pruning
    fn mdp(&self, ply: Ply, bounds: &Range<Score>) -> (Score, Score) {
        let lower = Score::mated(ply);
        let upper = Score::mating(ply + 1); // One can't mate in 0 plies!
        (bounds.start.max(lower), bounds.end.min(upper))
    }

    /// An implementation of [null move pruning].
    ///
    /// [null move pruning]: https://www.chessprogramming.org/Null_Move_Pruning
    fn nmp(&self, surplus: Score, draft: Depth) -> Option<Depth> {
        match surplus.get() {
            0.. => Some(draft - 2 - draft / 4),
            ..0 => None,
        }
    }

    /// An implementation of [multi-cut pruning].
    ///
    /// [multi-cut pruning]: https://www.chessprogramming.org/Multi-Cut
    fn mcp(&self, surplus: Score, draft: Depth) -> Option<Depth> {
        match surplus.get() {
            0.. if draft >= 6 => Some(draft / 2),
            _ => None,
        }
    }

    /// An implementation of [reverse futility pruning].
    ///
    /// [reverse futility pruning]: https://www.chessprogramming.org/Reverse_Futility_Pruning
    fn rfp(&self, surplus: Score, draft: Depth) -> Option<Depth> {
        match surplus.get() {
            ..0 => None,
            0..680 => Some(draft - (surplus + 40) / 120),
            680.. => Some(draft - 6),
        }
    }

    /// An implementation of [futility pruning].
    ///
    /// [futility pruning]: https://www.chessprogramming.org/Futility_Pruning
    fn fp(&self, deficit: Score, draft: Depth) -> Option<Depth> {
        let r = match deficit.get() {
            ..15 => return None,
            15..50 => 1,
            50..100 => 2,
            100.. => 3,
        };

        Some(draft - r - draft / 4)
    }

    /// An implementation of [late move reductions].
    ///
    /// [late move reductions]: https://www.chessprogramming.org/Late_Move_Reductions
    fn lmr(&self, draft: Depth, idx: usize) -> i8 {
        draft.get().max(1).ilog2() as i8 * idx.max(1).ilog2() as i8 / 3
    }

    /// The [alpha-beta] search.
    ///
    /// [alpha-beta]: https://www.chessprogramming.org/Alpha-Beta
    fn ab<const N: usize>(
        &self,
        pos: &Evaluator,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        ctrl: &Control,
    ) -> Result<Pv<N>, Interrupted> {
        if ply.cast::<usize>() < N && depth > ply && bounds.start + 1 < bounds.end {
            self.pvs(pos, bounds, depth, ply, ctrl)
        } else {
            Ok(self.pvs::<0>(pos, bounds, depth, ply, ctrl)?.convert())
        }
    }

    /// The [zero-window] alpha-beta search.
    ///
    /// [zero-window]: https://www.chessprogramming.org/Null_Window
    fn nw<const N: usize>(
        &self,
        pos: &Evaluator,
        beta: Score,
        depth: Depth,
        ply: Ply,
        ctrl: &Control,
    ) -> Result<Pv<N>, Interrupted> {
        self.ab(pos, beta - 1..beta, depth, ply, ctrl)
    }

    /// An implementation of the [PVS] variation of the alpha-beta search.
    ///
    /// [PVS]: https://www.chessprogramming.org/Principal_Variation_Search
    fn pvs<const N: usize>(
        &self,
        pos: &Evaluator,
        bounds: Range<Score>,
        depth: Depth,
        ply: Ply,
        ctrl: &Control,
    ) -> Result<Pv<N>, Interrupted> {
        ctrl.interrupted()?;
        let is_root = ply == 0;
        (bounds.start < bounds.end).assume();
        let (alpha, beta) = match pos.outcome() {
            None => self.mdp(ply, &bounds),
            Some(Outcome::DrawByThreefoldRepetition) if is_root => self.mdp(ply, &bounds),
            Some(o) if o.is_draw() => return Ok(Pv::new(Score::new(0), [])),
            Some(_) => return Ok(Pv::new(Score::mated(ply), [])),
        };

        if alpha >= beta {
            return Ok(Pv::new(alpha, []));
        }

        let transposition = self.tt.get(pos.zobrist());
        let transposed = match transposition {
            None => Pv::new(pos.evaluate().saturate(), []),
            Some(t) => t.transpose(ply),
        };

        #[cfg(not(test))]
        let mut depth = depth;

        #[cfg(not(test))]
        let mut alpha = alpha;

        if !is_root {
            #[cfg(not(test))]
            if transposition.is_some() && pos.is_check() {
                // The check extension heuristic is not exact.
                depth = depth + 1;
            }

            #[cfg(not(test))]
            if transposition.is_none() && !pos.is_check() {
                // The internal iterative reduction heuristic is not exact.
                depth = depth - 2;
            }
        }

        let draft = depth - ply;
        let quiesce = draft <= 0;
        let is_pv = alpha + 1 < beta;
        if let Some(t) = transposition {
            if !is_pv && t.draft() >= draft {
                let (lower, upper) = t.score().range(ply).into_inner();
                if lower >= upper || upper <= alpha || lower >= beta {
                    return Ok(transposed.convert());
                }
            }
        }

        #[cfg(not(test))]
        if quiesce {
            // The stand pat heuristic is not exact.
            alpha = transposed.score().max(alpha);
        }

        if alpha >= beta || ply >= Ply::MAX {
            return Ok(transposed.convert());
        }

        if let Some(t) = transposition {
            if let Some(d) = self.rfp(t.score().lower(ply) - beta, draft) {
                if !is_pv && t.draft() >= d {
                    #[cfg(not(test))]
                    // The reverse futility pruning heuristic is not exact.
                    return Ok(transposed.convert());
                }
            }
        }

        if let Some(d) = self.nmp(transposed.score() - beta, draft) {
            if !is_pv && !pos.is_check() && pos.pieces(pos.turn()).len() > 1 {
                if d <= 0 {
                    #[cfg(not(test))]
                    // The null move pruning heuristic is not exact.
                    return Ok(transposed.convert());
                } else {
                    let mut next = pos.clone();
                    next.pass();
                    self.tt.prefetch(next.zobrist());
                    if -self.nw::<0>(&next, -beta + 1, d + ply, ply + 1, ctrl)? >= beta {
                        #[cfg(not(test))]
                        // The null move pruning heuristic is not exact.
                        return Ok(transposed.convert());
                    }
                }
            }
        }

        let killers = self.killers.get(ply, pos.turn());
        let mut moves: ArrayVec<_, 255> = pos
            .moves()
            .filter(|ms| !quiesce || !ms.is_quiet())
            .flatten()
            .map(|m| {
                if Some(m) == transposed.moves().next() {
                    (m, Value::upper())
                } else if killers.contains(m) {
                    (m, Value::new(25))
                } else if m.is_quiet() {
                    (m, Value::lower() / 2 + self.history.get(m, pos.turn()))
                } else {
                    let mut next = pos.material();
                    let material = next.evaluate();
                    next.play(m);
                    (m, -next.evaluate() - material)
                }
            })
            .collect();

        moves.sort_unstable_by_key(|(_, gain)| *gain);

        if let Some(t) = transposition {
            if let Some(d) = self.mcp(t.score().lower(ply) - beta, draft) {
                if !is_root && t.draft() >= d {
                    for (m, _) in moves.iter().rev().skip(1) {
                        let mut next = pos.clone();
                        next.play(*m);
                        self.tt.prefetch(next.zobrist());
                        if -self.nw::<0>(&next, -beta + 1, d + ply, ply + 1, ctrl)? >= beta {
                            #[cfg(not(test))]
                            // The multi-cut pruning heuristic is not exact.
                            return Ok(transposed.convert());
                        }
                    }

                    #[cfg(not(test))]
                    {
                        // The singular extension heuristic is not exact.
                        depth = depth + 1;
                    }
                }
            }
        }

        let (head, tail) = match moves.pop() {
            None => return Ok(transposed.convert()),
            Some((m, _)) => {
                let mut next = pos.clone();
                next.play(m);
                self.tt.prefetch(next.zobrist());
                (m, -self.ab(&next, -beta..-alpha, depth, ply + 1, ctrl)?)
            }
        };

        if tail >= beta || moves.is_empty() {
            self.record(pos, &[], bounds, depth, ply, head, tail.score());
            return Ok(head >> tail);
        }

        let (head, tail) = self.driver.drive(head, tail, &moves, |score, m, gain, n| {
            let alpha = match score {
                s if s >= beta => return Err(ControlFlow::Break),
                s => s.max(alpha),
            };

            let mut next = pos.clone();
            next.play(m);

            self.tt.prefetch(next.zobrist());
            if gain <= Value::lower() / 2 && !pos.is_check() && !next.is_check() {
                if let Some(d) = self.fp(alpha + next.evaluate(), draft) {
                    if d <= 0 || -self.nw::<0>(&next, -alpha, d + ply, ply + 1, ctrl)? <= alpha {
                        #[cfg(not(test))]
                        // The futility pruning heuristic is not exact.
                        return Err(ControlFlow::Continue);
                    }
                }
            }

            let lmr = match self.lmr(draft, n) {
                #[cfg(not(test))]
                // The late move reduction heuristic is not exact.
                r @ 1.. => r - (is_pv as i8),
                _ => 0,
            };

            let partial = match -self.nw(&next, -alpha, depth - lmr, ply + 1, ctrl)? {
                partial if partial <= alpha || (partial >= beta && lmr <= 0) => partial,
                _ => -self.ab(&next, -beta..-alpha, depth, ply + 1, ctrl)?,
            };

            Ok(partial)
        })?;

        self.record(pos, &moves, bounds, depth, ply, head, tail.score());
        Ok(head >> tail)
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
        time: &Range<Duration>,
        stopper: &Trigger,
    ) -> Pv {
        let ctrl = Control::Limited(Counter::new(nodes), Timer::new(time.end), stopper);
        let mut pv = Pv::new(Score::lower(), []);

        'id: for depth in Depth::iter() {
            let mut draft = depth;
            let mut delta = 5i16;

            let (mut lower, mut upper) = match depth.get() {
                ..=4 => (Score::lower(), Score::upper()),
                _ => (pv.score() - delta, pv.score() + delta),
            };

            let ctrl = if pv.moves().next().is_none() {
                &Control::Unlimited
            } else if depth < limit {
                &ctrl
            } else {
                break 'id;
            };

            'aw: loop {
                delta = delta.saturating_mul(2);
                if ctrl.timer().remaining() < Some(time.end - time.start) {
                    break 'id;
                }

                let Ok(partial) = self.ab(pos, lower..upper, draft, Ply::new(0), ctrl) else {
                    break 'id;
                };

                match partial.score() {
                    score if (-lower..Score::upper()).contains(&-score) => {
                        upper = lower / 2 + upper / 2;
                        lower = score - delta;
                        draft = depth;
                    }

                    score if (upper..Score::upper()).contains(&score) => {
                        upper = score + delta;
                        pv = partial;

                        #[cfg(not(test))]
                        {
                            // Reductions are not exact.
                            draft = draft - 1;
                        }
                    }

                    _ => {
                        pv = partial;
                        break 'aw;
                    }
                }
            }
        }

        pv
    }

    fn time_to_search(&self, pos: &Evaluator, limits: &Limits) -> Range<Duration> {
        let (clock, inc) = match limits {
            Limits::Clock(c, i) => (c, i),
            _ => return limits.time()..limits.time(),
        };

        let time_left = clock.saturating_sub(*inc);
        let moves_left = 280 / pos.fullmoves().get().min(40);
        let time_per_move = inc.saturating_add(time_left / moves_left);
        time_per_move / 2..time_per_move
    }

    /// Searches for the [principal variation][`Pv`].
    pub fn search(&mut self, pos: &Evaluator, limits: &Limits, stopper: &Trigger) -> Pv {
        let time = self.time_to_search(pos, limits);
        self.aw(pos, limits.depth(), limits.nodes(), &time, stopper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::Move;
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    fn alphabeta(pos: &Evaluator, bounds: Range<Score>, depth: Depth, ply: Ply) -> Score {
        let (mut alpha, beta) = (bounds.start, bounds.end);
        debug_assert!(alpha < beta);

        let score = match pos.outcome() {
            Some(o) if o.is_draw() => return Score::new(0),
            Some(_) => return Score::mated(ply),
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

    #[proptest]
    fn hash_is_an_upper_limit_for_table_size(o: Options) {
        let e = Engine::with_options(&o);
        prop_assume!(e.tt.capacity() > 1);
        assert!(e.tt.size() <= o.hash);
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
        use Control::Unlimited;
        let tpos = Transposition::new(ScoreBound::Lower(s), d, m);
        e.tt.set(pos.zobrist(), tpos);
        assert_eq!(e.nw::<1>(&pos, b, d, p, &Unlimited), Ok(Pv::new(s, [])));
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
        use Control::Unlimited;
        let tpos = Transposition::new(ScoreBound::Upper(s), d, m);
        e.tt.set(pos.zobrist(), tpos);
        assert_eq!(e.nw::<1>(&pos, b, d, p, &Unlimited), Ok(Pv::new(s, [])));
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
        #[filter(#s.mate().is_none())] s: Score,
        #[map(|s: Selector| s.select(#pos.moves().flatten()))] m: Move,
    ) {
        use Control::Unlimited;
        let tpos = Transposition::new(ScoreBound::Exact(s), d, m);
        e.tt.set(pos.zobrist(), tpos);
        assert_eq!(e.nw::<1>(&pos, b, d, p, &Unlimited), Ok(Pv::new(s, [])));
    }

    #[proptest]
    fn nw_finds_score_bound(
        #[by_ref] e: Engine,
        pos: Evaluator,
        #[filter((Value::lower()..Value::upper()).contains(&#b))] b: Score,
        d: Depth,
        #[filter(#p > 0)] p: Ply,
    ) {
        assert_eq!(
            e.nw::<1>(&pos, b, d, p, &Control::Unlimited)? < b,
            alphabeta(&pos, b - 1..b, d, p) < b
        );
    }

    #[proptest]
    fn ab_aborts_if_maximum_number_of_nodes_visited(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        #[filter(#p > 0)] p: Ply,
    ) {
        let trigger = Trigger::armed();
        let ctrl = Control::Limited(Counter::new(0), Timer::infinite(), &trigger);
        assert_eq!(e.ab::<1>(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn ab_aborts_if_time_is_up(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        #[filter(#p > 0)] p: Ply,
    ) {
        let trigger = Trigger::armed();
        let ctrl = Control::Limited(Counter::new(u64::MAX), Timer::new(Duration::ZERO), &trigger);
        std::thread::sleep(Duration::from_millis(1));
        assert_eq!(e.ab::<1>(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn ab_aborts_if_stopper_is_disarmed(
        e: Engine,
        pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        #[filter(#p > 0)] p: Ply,
    ) {
        let trigger = Trigger::disarmed();
        let ctrl = Control::Limited(Counter::new(u64::MAX), Timer::infinite(), &trigger);
        assert_eq!(e.ab::<1>(&pos, b, d, p, &ctrl), Err(Interrupted));
    }

    #[proptest]
    fn ab_returns_static_evaluation_if_max_ply(
        e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
    ) {
        assert_eq!(
            e.ab::<1>(&pos, b, d, Ply::upper(), &Control::Unlimited),
            Ok(Pv::new(pos.evaluate().saturate(), []))
        );
    }

    #[proptest]
    fn ab_returns_drawn_score_if_game_ends_in_a_draw(
        #[by_ref] e: Engine,
        #[filter(#pos.outcome().is_some_and(|o| o.is_draw()))] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        #[filter(#p > 0 || #pos.outcome() != Some(Outcome::DrawByThreefoldRepetition))] p: Ply,
    ) {
        assert_eq!(
            e.ab::<1>(&pos, b, d, p, &Control::Unlimited),
            Ok(Pv::new(Score::new(0), []))
        );
    }

    #[proptest]
    fn ab_returns_lost_score_if_game_ends_in_checkmate(
        e: Engine,
        #[filter(#pos.outcome().is_some_and(|o| o.is_decisive()))] pos: Evaluator,
        #[filter(!#b.is_empty())] b: Range<Score>,
        d: Depth,
        #[filter(#p > 0)] p: Ply,
    ) {
        assert_eq!(
            e.ab::<1>(&pos, b, d, p, &Control::Unlimited),
            Ok(Pv::new(Score::mated(p), []))
        );
    }

    #[proptest]
    fn search_finds_the_minimax_score(mut e: Engine, pos: Evaluator, #[filter(#d > 1)] d: Depth) {
        let trigger = Trigger::armed();
        let time = Duration::MAX..Duration::MAX;

        assert_eq!(
            e.search(&pos, &Limits::Depth(d), &trigger).score(),
            e.aw(&pos, d, u64::MAX, &time, &trigger).score()
        );
    }

    #[proptest]
    fn search_is_stable(mut e: Engine, pos: Evaluator, d: Depth) {
        let limits = Limits::Depth(d);
        let trigger = Trigger::armed();

        assert_eq!(
            e.search(&pos, &limits, &trigger).score(),
            e.search(&pos, &limits, &trigger).score()
        );
    }

    #[proptest]
    fn search_extends_time_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        let limits = Duration::ZERO.into();
        let trigger = Trigger::armed();
        assert_ne!(e.search(&pos, &limits, &trigger).moves().next(), None);
    }

    #[proptest]
    fn search_extends_depth_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        let limits = Depth::lower().into();
        let trigger = Trigger::armed();
        assert_ne!(e.search(&pos, &limits, &trigger).moves().next(), None);
    }

    #[proptest]
    fn search_ignores_stopper_to_find_some_pv(
        mut e: Engine,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
    ) {
        let limits = Limits::None;
        let trigger = Trigger::armed();
        assert_ne!(e.search(&pos, &limits, &trigger).moves().next(), None);
    }
}
