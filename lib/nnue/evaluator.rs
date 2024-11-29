use crate::chess::{Color, Move, ParsePositionError, Perspective, Piece, Position, Role, Square};
use crate::nnue::{Accumulator, Feature, Material, Positional, Value};
use crate::util::Integer;
use arrayvec::ArrayVec;
use derive_more::{Debug, Deref, Display};
use std::str::FromStr;

#[cfg(test)]
use proptest::prelude::*;

/// An incrementally evaluated [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Deref)]
#[debug("Evaluator({self})")]
#[display("{pos}")]
pub struct Evaluator<T: Accumulator = (Material, Positional)> {
    #[deref]
    pos: Position,
    acc: T,
}

#[cfg(test)]
impl<T: 'static + Accumulator> Arbitrary for Evaluator<T> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        any::<Position>().prop_map(Evaluator::new).boxed()
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new(Position::default())
    }
}

impl<T: Accumulator> Evaluator<T> {
    /// Constructs the evaluator from a [`Position`].
    pub fn new(pos: Position) -> Self {
        let mut acc = T::default();
        for side in Color::iter() {
            let ksq = pos.king(side);
            for (p, s) in pos.iter() {
                acc.add(side, Feature::new(side, ksq, p, s));
            }
        }

        Evaluator { pos, acc }
    }

    /// The [`Position`]'s evaluation.
    pub fn evaluate(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        let value = self.acc.evaluate(self.turn(), phase) >> 7;
        value.saturate()
    }

    /// Play a [null-move].
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    pub fn pass(&mut self) {
        self.pos.pass();
    }

    /// Play a [`Move`].
    pub fn play(&mut self, m: Move) {
        let turn = self.turn();
        let promotion = m.promotion();
        let (wc, wt) = (m.whence(), m.whither());
        let (role, capture) = self.pos.play(m);
        let mut sides = ArrayVec::<Color, 2>::from([!turn, turn]);

        if role == Role::King
            && Feature::new(turn, wc, Piece::lower(), Square::lower())
                != Feature::new(turn, wt, Piece::lower(), Square::lower())
        {
            sides.truncate(1);
            self.acc.refresh(turn);
            for (p, s) in self.pos.iter() {
                self.acc.add(turn, Feature::new(turn, wt, p, s));
            }
        }

        for side in sides {
            let ksq = self.king(side);
            let old = Piece::new(role, turn);
            let new = Piece::new(promotion.unwrap_or(role), turn);
            self.acc.replace(
                side,
                Feature::new(side, ksq, old, wc),
                Feature::new(side, ksq, new, wt),
            );

            if let Some((r, s)) = capture {
                let victim = Piece::new(r, !turn);
                self.acc.remove(side, Feature::new(side, ksq, victim, s));
            } else if role == Role::King && (wt - wc).abs() == 2 {
                let rook = Piece::new(Role::Rook, turn);
                let (wc, wt) = if wt > wc {
                    (Square::H1.perspective(turn), Square::F1.perspective(turn))
                } else {
                    (Square::A1.perspective(turn), Square::D1.perspective(turn))
                };

                self.acc.replace(
                    side,
                    Feature::new(side, ksq, rook, wc),
                    Feature::new(side, ksq, rook, wt),
                );
            }
        }
    }
}

impl Evaluator {
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

impl FromStr for Evaluator {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.parse()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::Selector;
    use std::fmt::Debug;
    use test_strategy::proptest;

    #[proptest]
    fn play_updates_evaluator(
        #[filter(#e.outcome().is_none())] mut e: Evaluator,
        #[map(|sq: Selector| sq.select(#e.moves().flatten()))] m: Move,
    ) {
        let mut pos = e.pos.clone();
        e.play(m);
        pos.play(m);
        assert_eq!(e, Evaluator::new(pos));
    }

    #[proptest]
    fn pass_updates_evaluator(#[filter(!#e.is_check())] mut e: Evaluator) {
        let mut pos = e.pos.clone();
        e.pass();
        pos.pass();
        assert_eq!(e, Evaluator::new(pos));
    }

    #[proptest]
    fn parsing_printed_evaluator_is_an_identity(e: Evaluator) {
        assert_eq!(e.to_string().parse(), Ok(e));
    }
}
