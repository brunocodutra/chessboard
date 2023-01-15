use crate::chess::{Color, Piece, Position, Role, Square};
use derive_more::Constructor;
use test_strategy::Arbitrary;

mod end;
mod mid;
mod value;

pub use end::*;
pub use mid::*;
pub use value::*;

/// Trait for types that can evaluate [`Position`]s.
pub trait Eval {
    /// Evaluates a [`Position`].
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, item: &Position) -> Value;
}

/// A tapered evaluator.
#[derive(Debug, Default, Clone, Arbitrary, Constructor)]
pub struct Evaluator();

impl Evaluator {
    const PHASES: usize = 24;
    const PIECE_WEIGHT: [usize; 6] = [0, 1, 1, 2, 4, 0];
    const PIECE_SQUARE_TABLE: [[[i16; 64]; 6]; Self::PHASES + 1] = {
        let mut table = [[[0; 64]; 6]; Self::PHASES + 1];

        let mut p = table.len();
        while p > 0 {
            p -= 1;
            let mut r = table[p].len();
            while r > 0 {
                r -= 1;
                let mut s = table[p][r].len();
                while s > 0 {
                    s -= 1;
                    let mg = MidGame::PIECE_VALUE[r] + MidGame::PIECE_SQUARE_BONUS[r][s];
                    let eg = EndGame::PIECE_VALUE[r] + EndGame::PIECE_SQUARE_BONUS[r][s];
                    table[p][r][s] = eg + p as i16 * (mg - eg) / Self::PHASES as i16;
                }
            }
        }

        table
    };

    fn phase(&self, pos: &Position) -> usize {
        Role::iter()
            .zip(Self::PIECE_WEIGHT)
            .map(|(r, w)| pos.by_role(r).len() * w)
            .sum::<usize>()
            .min(Self::PHASES)
    }

    fn lookup(&self, phase: usize, Piece(c, r): Piece, s: Square) -> i16 {
        Self::PIECE_SQUARE_TABLE[phase][r as usize][match c {
            Color::White => s.mirror().index() as usize,
            Color::Black => s.index() as usize,
        }]
    }
}

impl Eval for Evaluator {
    fn eval(&self, pos: &Position) -> Value {
        let turn = pos.turn();
        let phase = self.phase(pos);

        let mut score = [0; 2];
        for r in Role::iter() {
            for c in [Color::White, Color::Black] {
                for s in pos.by_piece(Piece(c, r)) {
                    score[c as usize] += self.lookup(phase, Piece(c, r), s);
                }
            }
        }

        Value::saturate(score[turn as usize] - score[!turn as usize])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn position_evaluation_is_symmetric(e: Evaluator, #[filter(!#pos.is_check())] pos: Position) {
        let mut mirror = pos.clone();
        mirror.pass()?;

        assert_eq!(e.eval(&pos), -e.eval(&mirror));
    }
}
