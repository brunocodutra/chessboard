use crate::chess::{Color, Move, ParsePositionError, Piece, Position, Role, Square};
use crate::nnue::{Accumulator, Feature, Material, Positional, Value};
use crate::util::Integer;
use arrayvec::ArrayVec;
use derive_more::{Debug, Deref, Display};
use std::{ops::Range, str::FromStr};

fn perspective(pos: &Position, side: Color) -> ArrayVec<u16, 32> {
    let k = pos.king(side);
    ArrayVec::from_iter(pos.iter().map(|(p, s)| Feature(k, p, s).index(side)))
}

/// An incrementally evaluated [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("Evaluator({self})")]
#[display("{pos}")]
pub struct Evaluator<T: Clone + Accumulator = (Material, Positional)> {
    #[deref]
    pos: Position,
    #[cfg_attr(test, map(|mut acc: T| {
        acc.refresh(
            &perspective(&#pos, Color::White),
            &perspective(&#pos, Color::Black),
        );
        acc
    }))]
    acc: T,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new(Position::default())
    }
}

impl FromStr for Evaluator {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.parse()?))
    }
}

impl Evaluator {
    /// Constructs the evaluator from a [`Position`].
    pub fn new(pos: Position) -> Self {
        let mut acc = <(Material, Positional)>::default();
        let white = perspective(&pos, Color::White);
        let black = perspective(&pos, Color::Black);
        acc.refresh(&white, &black);
        Evaluator { pos, acc }
    }

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

impl<T: Clone + Accumulator> Evaluator<T> {
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
        let (role, capture) = self.pos.play(m);
        let turn = self.turn();

        use Color::*;
        if role == Role::King {
            let white = perspective(&self.pos, White);
            let black = perspective(&self.pos, Black);
            self.acc.refresh(&white, &black);
        } else {
            let kings = [self.pos.king(White), self.pos.king(Black)];

            let new = Piece::new(m.promotion().unwrap_or(role), !turn);
            let fts = kings.map(|ks| Feature(ks, new, m.whither()));
            self.acc.add(fts[0].index(White), fts[1].index(Black));

            let old = Piece::new(role, !turn);
            let fts = kings.map(|ks| Feature(ks, old, m.whence()));
            self.acc.remove(fts[0].index(White), fts[1].index(Black));

            if let Some((v, s)) = capture {
                let fts = kings.map(|ks| Feature(ks, Piece::new(v, turn), s));
                self.acc.remove(fts[0].index(White), fts[1].index(Black));
            }
        }
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
        assert!((a..=b).contains(&Evaluator::new(pos).see(sq, r)));
    }

    #[proptest]
    fn see_returns_beta_if_alpha_is_not_smaller(pos: Position, sq: Square, r: Range<Value>) {
        assert_eq!(Evaluator::new(pos).see(sq, r.end..r.start), r.start);
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
