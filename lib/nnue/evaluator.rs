use crate::chess::{Color, Move, ParsePositionError, Piece, Position, Role, Square};
use crate::nnue::{Accumulator, Feature, Material, Positional, Value};
use crate::util::Integer;
use derive_more::{Debug, Deref, Display};
use std::{ops::Range, str::FromStr};

#[cfg(test)]
use proptest::prelude::*;

/// An incrementally evaluated [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Deref)]
#[debug("Evaluator({self})")]
#[display("{pos}")]
pub struct Evaluator<T: Clone + Accumulator = (Material, Positional)> {
    #[deref]
    pos: Position,
    acc: T,
}

#[cfg(test)]
impl<T: 'static + Clone + Accumulator> Arbitrary for Evaluator<T> {
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

impl<T: Clone + Accumulator> Evaluator<T> {
    /// Constructs the evaluator from a [`Position`].
    #[inline(always)]
    pub fn new(pos: Position) -> Self {
        let ksqs = [Color::White, Color::Black].map(|c| (c, pos.king(c)));

        let mut acc = T::default();
        for (p, s) in pos.iter() {
            let fts = ksqs.map(|(c, ksq)| Feature::new(c, ksq, p, s));
            acc.add(fts[0], fts[1]);
        }

        Evaluator { pos, acc }
    }

    /// The [`Position`]'s evaluation.
    pub fn evaluate(&self) -> Value {
        let phase = (self.occupied().len() - 1) / 4;
        self.acc.evaluate(self.turn(), phase).saturate()
    }

    /// The Static Exchange Evaluation ([SEE]) algorithm.
    ///
    /// [SEE]: https://www.chessprogramming.org/Static_Exchange_Evaluation
    pub fn see(&mut self, sq: Square, bounds: Range<Value>) -> Value {
        let (mut alpha, mut beta) = (bounds.start, bounds.end);

        loop {
            alpha = alpha.max(self.evaluate());

            if alpha >= beta {
                break beta;
            }

            let Some(m) = self.exchange(sq) else {
                break alpha;
            };

            self.play(m);

            beta = beta.min(-self.evaluate());

            if alpha >= beta {
                break alpha;
            }

            let Some(m) = self.exchange(sq) else {
                break beta;
            };

            self.play(m);
        }
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
        let (role, capture) = self.pos.play(m);

        if role == Role::King {
            *self = Evaluator::new(self.pos)
        } else {
            let ksqs = [Color::White, Color::Black].map(|c| (c, self.pos.king(c)));

            let old = Piece::new(role, turn);
            let new = Piece::new(m.promotion().unwrap_or(role), turn);
            let remove = ksqs.map(|(c, ksq)| Feature::new(c, ksq, old, m.whence()));
            let add = ksqs.map(|(c, ksq)| Feature::new(c, ksq, new, m.whither()));
            self.acc.replace([remove[0], add[0]], [remove[1], add[1]]);

            if let Some((r, s)) = capture {
                let victim = Piece::new(r, !turn);
                let fts = ksqs.map(|(c, ksq)| Feature::new(c, ksq, victim, s));
                self.acc.remove(fts[0], fts[1]);
            }
        }
    }
}

impl Evaluator {
    /// The [`Position`]'s material evaluator.
    pub fn material(&self) -> Evaluator<Material> {
        Evaluator {
            pos: self.pos,
            acc: self.acc.0.clone(),
        }
    }

    /// The [`Position`]'s positional evaluator.
    pub fn positional(&self) -> Evaluator<Positional> {
        Evaluator {
            pos: self.pos,
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
    fn see_returns_value_within_bounds(pos: Position, sq: Square, r: Range<Value>) {
        let (a, b) = (r.start, r.end);
        assert!((a..=b).contains(&Evaluator::<Positional>::new(pos).see(sq, r)));
    }

    #[proptest]
    fn see_returns_beta_if_alpha_is_not_smaller(pos: Position, sq: Square, r: Range<Value>) {
        assert_eq!(
            Evaluator::<Positional>::new(pos).see(sq, r.end..r.start),
            r.start
        );
    }

    #[proptest]
    fn play_updates_evaluator(
        #[filter(#e.outcome().is_none())] mut e: Evaluator,
        #[map(|sq: Selector| sq.select(#e.moves().flatten()))] m: Move,
    ) {
        let mut pos = e.pos;
        e.play(m);
        pos.play(m);
        assert_eq!(e, Evaluator::new(pos));
    }

    #[proptest]
    fn pass_updates_evaluator(#[filter(!#e.is_check())] mut e: Evaluator) {
        let mut pos = e.pos;
        e.pass();
        pos.pass();
        assert_eq!(e, Evaluator::new(pos));
    }

    #[proptest]
    fn parsing_printed_evaluator_is_an_identity(e: Evaluator) {
        assert_eq!(e.to_string().parse(), Ok(e));
    }
}
