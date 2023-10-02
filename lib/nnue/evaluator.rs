use crate::chess::{Color, Move, MoveContext, Piece, Position, Role, Square};
use crate::chess::{IllegalMove, ImpossibleExchange, ImpossiblePass};
use crate::nnue::{Feature, Layer, Nnue, Vector, NNUE};
use crate::{search::Value, util::Buffer};
use derive_more::Deref;
use std::{borrow::Cow, iter::repeat, mem::transmute, ops::Range};

/// An incrementally evaluated [`Position`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
pub struct Evaluator<'a> {
    #[deref(forward)]
    pos: Cow<'a, Position>,
    hidden: [Vector<i16, { Nnue::L1 / 2 }>; 2],
    psqt: [Vector<i32, { Nnue::PHASES }>; 2],
}

impl<'a> Evaluator<'a> {
    fn perspective(pos: &Position, side: Color) -> Buffer<u16, 32> {
        pos.iter()
            .zip(repeat(pos.king(side)))
            .map(|((p, s), ks)| Feature(ks, p, s))
            .map(|f| f.index(side))
            .collect()
    }

    fn new(pos: Cow<'a, Position>) -> Self {
        let us = Self::perspective(&pos, pos.turn());
        let them = Self::perspective(&pos, !pos.turn());
        let psqt = [NNUE.psqt.forward(&us), NNUE.psqt.forward(&them)];
        let hidden = [
            NNUE.transformer.forward(&us),
            NNUE.transformer.forward(&them),
        ];

        Evaluator { pos, hidden, psqt }
    }

    /// Constructs the accumulator from a borrowed [`Position`].
    pub fn borrow(pos: &'a Position) -> Self {
        Self::new(Cow::Borrowed(pos))
    }

    /// Constructs the accumulator from an owned [`Position`].
    pub fn own(pos: Position) -> Self {
        Self::new(Cow::Owned(pos))
    }

    /// The [`Position`]'s material evaluation.
    pub fn material(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        Value::saturate((self.psqt[0][phase] - self.psqt[1][phase]) / 32)
    }

    /// The [`Position`]'s positional evaluation.
    pub fn positional(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        let l1: &Vector<i16, { Nnue::L1 }> = unsafe { transmute(&self.hidden) };
        Value::saturate(NNUE.nns[phase].forward(l1) / 16)
    }

    /// The [`Position`]'s evaluation.
    pub fn value(&self) -> Value {
        self.material() + self.positional()
    }

    /// The Static Exchange Evaluation ([SEE]) algorithm.
    ///
    /// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
    pub fn see(&mut self, square: Square, bounds: Range<Value>) -> Value {
        assert!(!bounds.is_empty(), "{bounds:?} ≠ ∅");

        let (alpha, beta) = (bounds.start, bounds.end);
        let alpha = self.value().max(alpha);

        if alpha >= beta {
            return beta;
        }

        match self.exchange(square) {
            Ok(_) => -self.see(square, -beta..-alpha),
            Err(_) => alpha,
        }
    }

    /// Play a [null-move] if legal in this position.
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    pub fn pass(&mut self) -> Result<(), ImpossiblePass> {
        self.pos.to_mut().pass()?;
        self.hidden.reverse();
        self.psqt.reverse();
        Ok(())
    }

    /// Play a [`Move`] if legal in this position.
    pub fn play(&mut self, m: Move) -> Result<MoveContext, IllegalMove> {
        let m = self.pos.to_mut().play(m)?;
        self.update(m);
        Ok(m)
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    ///
    /// This may lead to invalid positions.
    pub fn exchange(&mut self, whither: Square) -> Result<MoveContext, ImpossibleExchange> {
        let m = self.pos.to_mut().exchange(whither)?;
        self.update(m);
        Ok(m)
    }

    fn update(&mut self, mc: MoveContext) {
        self.hidden.reverse();
        self.psqt.reverse();

        let turn = self.turn();
        let hidden = &mut self.hidden;
        let psqt = &mut self.psqt;

        if mc.role() == Role::King {
            let us = Self::perspective(&self.pos, turn);
            let them = Self::perspective(&self.pos, !turn);
            NNUE.transformer.refresh(&us, &mut hidden[0]);
            NNUE.transformer.refresh(&them, &mut hidden[1]);
            NNUE.psqt.refresh(&us, &mut psqt[0]);
            NNUE.psqt.refresh(&them, &mut psqt[1]);
        } else {
            let kings = [self.pos.king(turn), self.pos.king(!turn)];

            let new = Piece(!turn, Option::from(mc.promotion()).unwrap_or(mc.role()));
            let fts = kings.map(|ks| Feature(ks, new, mc.whither()));
            NNUE.transformer.add(fts[0].index(turn), &mut hidden[0]);
            NNUE.transformer.add(fts[1].index(!turn), &mut hidden[1]);
            NNUE.psqt.add(fts[0].index(turn), &mut psqt[0]);
            NNUE.psqt.add(fts[1].index(!turn), &mut psqt[1]);

            let old = Piece(!turn, mc.role());
            let fts = kings.map(|ks| Feature(ks, old, mc.whence()));
            NNUE.transformer.remove(fts[0].index(turn), &mut hidden[0]);
            NNUE.transformer.remove(fts[1].index(!turn), &mut hidden[1]);
            NNUE.psqt.remove(fts[0].index(turn), &mut psqt[0]);
            NNUE.psqt.remove(fts[1].index(!turn), &mut psqt[1]);

            if let Some((role, square)) = mc.capture() {
                let fts = kings.map(|ks| Feature(ks, Piece(turn, role), square));
                NNUE.transformer.remove(fts[0].index(turn), &mut hidden[0]);
                NNUE.transformer.remove(fts[1].index(!turn), &mut hidden[1]);
                NNUE.psqt.remove(fts[0].index(turn), &mut psqt[0]);
                NNUE.psqt.remove(fts[1].index(!turn), &mut psqt[1]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    #[proptest]
    fn material_evaluation_is_symmetric(#[filter(#pos.clone().pass().is_ok())] pos: Position) {
        let mut mirror = pos.clone();
        assert_eq!(mirror.pass(), Ok(()));
        assert_eq!(
            Evaluator::own(pos).material(),
            -Evaluator::own(mirror).material()
        );
    }

    #[proptest]
    fn see_returns_value_within_bounds(
        pos: Position,
        s: Square,
        #[filter(!#r.is_empty())] r: Range<Value>,
    ) {
        let (a, b) = (r.start, r.end);
        assert!((a..=b).contains(&Evaluator::own(pos).see(s, r)));
    }

    #[proptest]
    fn play_updates_accumulator(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        #[map(|s: Selector| *s.select(#pos.moves()))] m: Move,
    ) {
        let mut e = Evaluator::own(pos.clone());
        assert_eq!(e.play(m), pos.play(m));
        assert_eq!(e, Evaluator::own(pos));
    }

    #[proptest]
    fn pass_updates_accumulator(#[filter(#pos.clone().pass().is_ok())] mut pos: Position) {
        let mut e = Evaluator::own(pos.clone());
        assert_eq!(e.pass(), pos.pass());
        assert_eq!(e, Evaluator::own(pos));
    }

    #[proptest]
    fn exchange_updates_accumulator(
        #[filter(#pos.moves().filter(MoveContext::is_capture).next().is_some())] mut pos: Position,
        #[map(|s: Selector| *s.select(#pos.moves().filter(MoveContext::is_capture)))] m: Move,
    ) {
        let mut e = Evaluator::own(pos.clone());

        // Skip en passant captures.
        prop_assume!(e.exchange(m.whither()).is_ok());

        pos.exchange(m.whither())?;
        assert_eq!(e, Evaluator::own(pos));
    }
}
