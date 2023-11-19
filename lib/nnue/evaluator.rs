use crate::chess::{Color, ImpossibleExchange, Move, Piece, Position, Role, Square};
use crate::nnue::{Accumulator, Feature, Material, Positional, Value};
use crate::util::{Assume, Buffer};
use derive_more::Deref;

fn perspective(pos: &Position, side: Color) -> Buffer<u16, 32> {
    let k = pos.king(side);
    Buffer::from_iter(pos.iter().map(|(p, s)| Feature(k, p, s).index(side)))
}

/// An incrementally evaluated [`Position`].
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Evaluator<T: Clone + Accumulator = (Material, Positional)> {
    #[deref]
    pos: Position,
    #[cfg_attr(test, map(|mut acc: T| {
        acc.refresh(
            &perspective(&#pos, #pos.turn()),
            &perspective(&#pos, !#pos.turn()),
        );
        acc
    }))]
    acc: T,
}

impl Evaluator {
    /// Constructs the evaluator from a [`Position`].
    pub fn new(pos: Position) -> Self {
        let mut acc = <(Material, Positional)>::default();

        acc.refresh(
            &perspective(&pos, pos.turn()),
            &perspective(&pos, !pos.turn()),
        );

        Evaluator { pos, acc }
    }

    /// The [`Position`]'s material evaluator.
    pub fn material(&self) -> Evaluator<Material> {
        Evaluator {
            pos: self.pos.clone(),
            acc: self.acc.0.clone(),
        }
    }

    /// The [`Position`]'s positional evaluator.
    pub fn positional(&self) -> Evaluator<Positional> {
        Evaluator {
            pos: self.pos.clone(),
            acc: self.acc.1.clone(),
        }
    }
}

impl<T: Clone + Accumulator> Evaluator<T> {
    /// The [`Position`]'s evaluation.
    pub fn evaluate(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        Value::saturate(self.acc.evaluate(phase))
    }

    /// The Static Exchange Evaluation ([SEE]) algorithm.
    ///
    /// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
    pub fn see(&mut self, square: Square) -> Value {
        let (mut alpha, mut beta) = (Value::LOWER, Value::UPPER);

        loop {
            alpha = alpha.max(self.evaluate());

            if alpha >= beta {
                break beta;
            } else if self.exchange(square).is_err() {
                break alpha;
            }

            beta = beta.min(-self.evaluate());

            if alpha >= beta {
                break alpha;
            } else if self.exchange(square).is_err() {
                break beta;
            }
        }
    }

    /// Play a [null-move].
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    pub fn pass(&mut self) {
        self.pos.pass();
        self.acc.mirror();
    }

    /// Play a [`Move`].
    pub fn play(&mut self, m: Move) {
        let capture = self.role_on(m.whither());
        self.pos.play(m);
        self.acc.mirror();
        self.update(m, capture);
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    ///
    /// This may lead to invalid positions.
    #[inline(always)]
    fn exchange(&mut self, whither: Square) -> Result<Move, ImpossibleExchange> {
        let capture = self.role_on(whither);
        let m = self.pos.exchange(whither)?;
        self.acc.mirror();
        self.update(m, capture);
        Ok(m)
    }

    #[inline(always)]
    fn update(&mut self, m: Move, capture: Option<Role>) {
        let turn = self.turn();

        let role = if m.is_promotion() {
            Role::Pawn
        } else {
            self.role_on(m.whither()).assume()
        };

        if role == Role::King {
            let us = perspective(&self.pos, turn);
            let them = perspective(&self.pos, !turn);
            self.acc.refresh(&us, &them);
        } else {
            let kings = [self.pos.king(turn), self.pos.king(!turn)];

            let new = Piece(!turn, m.promotion().unwrap_or(role));
            let fts = kings.map(|ks| Feature(ks, new, m.whither()));
            self.acc.add(fts[0].index(turn), fts[1].index(!turn));

            let old = Piece(!turn, role);
            let fts = kings.map(|ks| Feature(ks, old, m.whence()));
            self.acc.remove(fts[0].index(turn), fts[1].index(!turn));

            if m.is_en_passant() {
                let target = Square::new(m.whither().file(), m.whence().rank());
                let fts = kings.map(|ks| Feature(ks, Piece(turn, Role::Pawn), target));
                self.acc.remove(fts[0].index(turn), fts[1].index(!turn));
            } else if let Some(role) = capture {
                let fts = kings.map(|ks| Feature(ks, Piece(turn, role), m.whither()));
                self.acc.remove(fts[0].index(turn), fts[1].index(!turn));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::Bitboard;
    use proptest::sample::Selector;
    use test_strategy::proptest;

    #[proptest]
    fn play_updates_evaluator(
        #[filter(#a.outcome().is_none())] mut a: Evaluator,
        #[map(|s: Selector| s.select(#a.moves(Bitboard::full())))] m: Move,
    ) {
        let mut b = a.pos.clone();
        a.play(m);
        b.play(m);
        assert_eq!(a, Evaluator::new(b));
    }

    #[proptest]
    fn pass_updates_evaluator(#[filter(!#a.is_check())] mut a: Evaluator) {
        let mut b = a.pos.clone();
        a.pass();
        b.pass();
        assert_eq!(a, Evaluator::new(b));
    }

    #[proptest]
    fn exchange_updates_evaluator(
        #[by_ref]
        #[filter(#a.moves(Bitboard::full()).filter(|m| m.is_capture() && !m.is_en_passant()).next().is_some())]
        mut a: Evaluator,
        #[map(|s: Selector| s.select(#a.moves(Bitboard::full()).filter(|m| m.is_capture() && !m.is_en_passant())).whither())]
        s: Square,
    ) {
        let mut b = a.pos.clone();
        a.exchange(s)?;
        b.exchange(s)?;
        assert_eq!(a, Evaluator::new(b));
    }
}
