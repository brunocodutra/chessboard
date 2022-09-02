use super::Eval;
use crate::chess::{Color, Piece, Position, Promotion, Role};

/// A trait for types that can evaluate positions using a [Piece-Square Table].
///
/// [Piece-Square Table]: https://www.chessprogramming.org/Piece-Square_Tables
pub trait PieceSquareTable {
    const PIECE_VALUE: [i16; 6];
    const PIECE_SQUARE_BONUS: [[i16; 64]; 6] = [[0; 64]; 6];
}

trait PrecomputedPieceSquareTable: PieceSquareTable {
    const PIECE_SQUARE_VALUE: [[i16; 64]; 6] = {
        let mut table = [[0; 64]; 6];

        let mut r = table.len();
        while r > 0 {
            r -= 1;
            let mut s = table[r].len();
            while s > 0 {
                s -= 1;
                table[r][s] = Self::PIECE_VALUE[r] + Self::PIECE_SQUARE_BONUS[r][s];
            }
        }

        table
    };
}

impl<T: PieceSquareTable> PrecomputedPieceSquareTable for T {}

impl<T: PieceSquareTable> Eval<Position> for T {
    fn eval(&self, pos: &Position) -> i16 {
        if pos.is_stalemate() || pos.is_material_insufficient() {
            0
        } else if pos.is_checkmate() {
            i16::MIN
        } else {
            let mut score = [0; 2];

            use Color::*;
            for c in [White, Black] {
                use Role::*;
                for r in [Pawn, Knight, Bishop, Rook, Queen, King] {
                    for s in pos.by_piece(Piece(c, r)) {
                        score[c as usize] += Self::PIECE_SQUARE_VALUE[r as usize][match c {
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

impl<T: PieceSquareTable> Eval<Role> for T {
    fn eval(&self, role: &Role) -> i16 {
        Self::PIECE_VALUE[*role as usize]
    }
}

impl<T: PieceSquareTable> Eval<Promotion> for T {
    fn eval(&self, p: &Promotion) -> i16 {
        Option::<Role>::from(*p).map(|r| self.eval(&r)).unwrap_or(0)
    }
}
