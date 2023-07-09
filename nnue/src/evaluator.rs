use crate::{Feature, Layer, Nnue, Transformer, NNUE};
use arrayvec::ArrayVec;
use chess::{Color, Move, MoveContext, Piece, Position, Role, Square};
use chess::{IllegalMove, ImpossibleExchange, ImpossiblePass};
use derive_more::Deref;
use std::mem::transmute_copy;
use std::{borrow::Cow, iter::repeat, ops::Range};
use util::Value;

/// An incrementally evaluated [`Position`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
pub struct Evaluator<'a> {
    #[deref(forward)]
    pos: Cow<'a, Position>,
    hidden: [[i16; Nnue::L1 / 2]; 2],
    psqt: [[i32; Nnue::PHASES]; 2],
}

impl<'a> Evaluator<'a> {
    #[inline]
    fn perspective(pos: &Position, side: Color) -> ArrayVec<usize, 32> {
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
    #[inline]
    pub fn borrow(pos: &'a Position) -> Self {
        Self::new(Cow::Borrowed(pos))
    }

    /// Constructs the accumulator from an owned [`Position`].
    #[inline]
    pub fn own(pos: Position) -> Self {
        Self::new(Cow::Owned(pos))
    }

    /// The [`Position`]'s material evaluation.
    #[inline]
    pub fn material(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        Value::saturate((self.psqt[0][phase] - self.psqt[1][phase]) / 32)
    }

    /// The [`Position`]'s positional evaluation.
    #[inline]
    pub fn positional(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        let l1: [i16; Nnue::L1] = unsafe { transmute_copy(&self.hidden) };
        Value::saturate(NNUE.nns[phase].forward(l1)[0] / 16)
    }

    /// The [`Position`]'s evaluation.
    #[inline]
    pub fn value(&self) -> Value {
        self.material() + self.positional()
    }

    /// The Static Exchange Evaluation ([SEE]) algorithm.
    ///
    /// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
    #[inline]
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
    #[inline]
    pub fn pass(&mut self) -> Result<(), ImpossiblePass> {
        self.pos.to_mut().pass()?;
        self.hidden.reverse();
        self.psqt.reverse();
        Ok(())
    }

    /// Play a [`Move`] if legal in this position.
    #[inline]
    pub fn play(&mut self, m: Move) -> Result<MoveContext, IllegalMove> {
        let m = self.pos.to_mut().play(m)?;
        self.update(m);
        Ok(m)
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    ///
    /// This may lead to invalid positions.
    #[inline]
    pub fn exchange(&mut self, whither: Square) -> Result<MoveContext, ImpossibleExchange> {
        let m = self.pos.to_mut().exchange(whither)?;
        self.update(m);
        Ok(m)
    }

    #[inline]
    fn update(&mut self, m: MoveContext) {
        self.hidden.reverse();
        self.psqt.reverse();

        let turn = self.turn();
        let hidden = &mut self.hidden;
        let psqt = &mut self.psqt;

        if m.role() == Role::King {
            let us = Self::perspective(&self.pos, turn);
            let them = Self::perspective(&self.pos, !turn);
            NNUE.transformer.refresh(&us, &mut hidden[0]);
            NNUE.transformer.refresh(&them, &mut hidden[1]);
            NNUE.psqt.refresh(&us, &mut psqt[0]);
            NNUE.psqt.refresh(&them, &mut psqt[1]);
        } else {
            let kings = [self.pos.king(turn), self.pos.king(!turn)];

            let new = Piece(!turn, Option::from(m.promotion()).unwrap_or(m.role()));
            let fts = kings.map(|ks| Feature(ks, new, m.whither()));
            NNUE.transformer.add(fts[0].index(turn), &mut hidden[0]);
            NNUE.transformer.add(fts[1].index(!turn), &mut hidden[1]);
            NNUE.psqt.add(fts[0].index(turn), &mut psqt[0]);
            NNUE.psqt.add(fts[1].index(!turn), &mut psqt[1]);

            let old = Piece(!turn, m.role());
            let fts = kings.map(|ks| Feature(ks, old, m.whence()));
            NNUE.transformer.remove(fts[0].index(turn), &mut hidden[0]);
            NNUE.transformer.remove(fts[1].index(!turn), &mut hidden[1]);
            NNUE.psqt.remove(fts[0].index(turn), &mut psqt[0]);
            NNUE.psqt.remove(fts[1].index(!turn), &mut psqt[1]);

            if let Some((role, square)) = m.capture() {
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
    use chess::MoveKind;
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    #[proptest]
    fn material_evaluation_is_symmetric(#[filter(!#pos.is_check())] pos: Position) {
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
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)] mut pos: Position,
        selector: Selector,
    ) {
        let m = *selector.select(pos.moves(MoveKind::ANY));
        let mut e = Evaluator::own(pos.clone());
        assert_eq!(e.play(m), pos.play(m));
        assert_eq!(e, Evaluator::own(pos));
    }

    #[proptest]
    fn pass_updates_accumulator(#[filter(!#pos.is_check())] mut pos: Position) {
        let mut e = Evaluator::own(pos.clone());
        assert_eq!(e.pass(), pos.pass());
        assert_eq!(e, Evaluator::own(pos));
    }

    #[proptest]
    fn exchange_updates_accumulator(
        #[filter(#pos.moves(MoveKind::CAPTURE).len() > 0)] mut pos: Position,
        selector: Selector,
    ) {
        let m = selector.select(pos.moves(MoveKind::CAPTURE));
        let mut e = Evaluator::own(pos.clone());

        // Skip en passant captures.
        prop_assume!(e.exchange(m.whither()).is_ok());

        pos.exchange(m.whither())?;
        assert_eq!(e, Evaluator::own(pos));
    }
}
