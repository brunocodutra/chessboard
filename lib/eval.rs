use crate::chess::{Color, Piece, Position, Promotion, Role};
use derive_more::Constructor;
use test_strategy::Arbitrary;

mod end;
mod mid;

pub use end::*;
pub use mid::*;

/// Trait for types that can evaluate other types.
pub trait Eval<T> {
    /// Evaluates an item.
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, item: &T) -> i16;
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
}

impl Eval<Position> for Evaluator {
    fn eval(&self, pos: &Position) -> i16 {
        if pos.is_stalemate() || pos.is_material_insufficient() {
            0
        } else if pos.is_checkmate() {
            i16::MIN
        } else {
            let mut score = [0; 2];

            let phase = self.phase(pos);

            for r in Role::iter() {
                for c in [Color::White, Color::Black] {
                    for s in pos.by_piece(Piece(c, r)) {
                        score[c as usize] += Self::PIECE_SQUARE_TABLE[phase][r as usize][match c {
                            Color::White => s.mirror().index() as usize,
                            Color::Black => s.index() as usize,
                        }];
                    }
                }
            }

            score[pos.turn() as usize] - score[!pos.turn() as usize]
        }
    }
}

impl Eval<Role> for Evaluator {
    fn eval(&self, role: &Role) -> i16 {
        EndGame::PIECE_VALUE[*role as usize]
    }
}

impl Eval<Promotion> for Evaluator {
    fn eval(&self, p: &Promotion) -> i16 {
        Option::<Role>::from(*p).map_or_else(|| 0, |r| self.eval(&r) - self.eval(&Role::Pawn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn position_evaluation_is_stable(pos: Position) {
        assert_eq!(Evaluator::new().eval(&pos), Evaluator::new().eval(&pos));
    }
}
