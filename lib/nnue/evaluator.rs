use crate::chess::{Color, Move, Piece, Position, Role, Square};
use crate::chess::{IllegalMove, ImpossibleExchange, ImpossiblePass};
use crate::nnue::{Accumulator, Feature, Material, Positional};
use crate::util::Assume;
use crate::{search::Value, util::Buffer};
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
    /// Constructs the accumulator from a [`Position`].
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

    /// Play a [null-move] if legal in this position.
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    pub fn pass(&mut self) -> Result<(), ImpossiblePass> {
        self.pos.pass()?;
        self.acc.mirror();
        Ok(())
    }

    /// Play a [`Move`] if legal in this position.
    pub fn play(&mut self, m: Move) -> Result<Move, IllegalMove> {
        let capture = self.role_on(m.whither());
        let m = self.pos.play(m)?;
        self.acc.mirror();
        self.update(m, capture);
        Ok(m)
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    ///
    /// This may lead to invalid positions.
    pub fn exchange(&mut self, whither: Square) -> Result<Move, ImpossibleExchange> {
        let capture = self.role_on(whither);
        let m = self.pos.exchange(whither)?;
        self.acc.mirror();
        self.update(m, capture);
        Ok(m)
    }

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
    use proptest::{prop_assume, sample::Selector};
    use test_strategy::proptest;

    #[proptest]
    fn play_updates_accumulator(
        #[filter(#a.outcome().is_none())] mut a: Evaluator,
        #[map(|s: Selector| s.select(#a.moves()))] m: Move,
    ) {
        let mut b = a.pos.clone();
        assert_eq!(a.play(m), b.play(m));
        assert_eq!(a, Evaluator::new(b));
    }

    #[proptest]
    fn pass_updates_accumulator(#[filter(#a.clone().pass().is_ok())] mut a: Evaluator) {
        let mut b = a.pos.clone();
        assert_eq!(a.pass(), b.pass());
        assert_eq!(a, Evaluator::new(b));
    }

    #[proptest]
    fn exchange_updates_accumulator(
        #[filter(#a.moves().filter(Move::is_capture).next().is_some())] mut a: Evaluator,
        #[map(|s: Selector| s.select(#a.moves().filter(Move::is_capture)))] m: Move,
    ) {
        let mut b = a.pos.clone();

        // Skip en passant captures.
        prop_assume!(b.exchange(m.whither()).is_ok());

        a.exchange(m.whither())?;
        assert_eq!(a, Evaluator::new(b));
    }
}
