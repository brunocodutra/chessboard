use crate::{Color, Eval, Game, Piece, Role};

/// A trait for types that can valuate material using a [Piece-Square Table].
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

impl<T: PieceSquareTable> Eval for T {
    fn eval(&self, game: &Game) -> i16 {
        let pos = game.position();
        let turn = pos.turn();

        match game.outcome() {
            Some(o) => match o.winner() {
                Some(w) if w == turn => i16::MAX,
                Some(_) => i16::MIN,
                None => 0,
            },

            None => {
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

                score[turn as usize] - score[!turn as usize]
            }
        }
    }
}
