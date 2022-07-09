use crate::{Color, Eval, Game, Piece, Role, Square};
use derive_more::Constructor;

/// [PeSTO]'s evaluation function.
///
/// [PeSTO]: https://www.chessprogramming.org/PeSTO%27s_Evaluation_Function
#[derive(Debug, Default, Constructor)]
pub struct Pesto {}

impl Pesto {
    const GAME_PHASE_PIECE_WEIGHT: [usize; 6] = [0, 1, 1, 2, 4, 0];

    const MID_GAME_PIECE_VALUE: [i16; 6] = [82, 337, 365, 477, 1025, 0];
    const END_GAME_PIECE_VALUE: [i16; 6] = [94, 281, 297, 512, 936, 0];

    #[rustfmt::skip]
    const MID_GAME_PAWN_SQUARE_BONUS: [i16; 64] = [
          0,   0,   0,   0,   0,   0,  0,   0,
         98, 134,  61,  95,  68, 126, 34, -11,
         -6,   7,  26,  31,  65,  56, 25, -20,
        -14,  13,   6,  21,  23,  12, 17, -23,
        -27,  -2,  -5,  12,  17,   6, 10, -25,
        -26,  -4,  -4, -10,   3,   3, 33, -12,
        -35,  -1, -20, -23, -15,  24, 38, -22,
          0,   0,   0,   0,   0,   0,  0,   0,
    ];

    #[rustfmt::skip]
    const END_GAME_PAWN_SQUARE_BONUS: [i16; 64] = [
          0,   0,   0,   0,   0,   0,   0,   0,
        178, 173, 158, 134, 147, 132, 165, 187,
         94, 100,  85,  67,  56,  53,  82,  84,
         32,  24,  13,   5,  -2,   4,  17,  17,
         13,   9,  -3,  -7,  -7,  -8,   3,  -1,
          4,   7,  -6,   1,   0,  -5,  -1,  -8,
         13,   8,   8,  10,  13,   0,   2,  -7,
          0,   0,   0,   0,   0,   0,   0,   0,
    ];

    #[rustfmt::skip]
    const MID_GAME_KNIGHT_SQUARE_BONUS: [i16; 64] = [
        -167, -89, -34, -49,  61, -97, -15, -107,
         -73, -41,  72,  36,  23,  62,   7,  -17,
         -47,  60,  37,  65,  84, 129,  73,   44,
          -9,  17,  19,  53,  37,  69,  18,   22,
         -13,   4,  16,  13,  28,  19,  21,   -8,
         -23,  -9,  12,  10,  19,  17,  25,  -16,
         -29, -53, -12,  -3,  -1,  18, -14,  -19,
        -105, -21, -58, -33, -17, -28, -19,  -23,
    ];

    #[rustfmt::skip]
    const END_GAME_KNIGHT_SQUARE_BONUS: [i16; 64] = [
        -58, -38, -13, -28, -31, -27, -63, -99,
        -25,  -8, -25,  -2,  -9, -25, -24, -52,
        -24, -20,  10,   9,  -1,  -9, -19, -41,
        -17,   3,  22,  22,  22,  11,   8, -18,
        -18,  -6,  16,  25,  16,  17,   4, -18,
        -23,  -3,  -1,  15,  10,  -3, -20, -22,
        -42, -20, -10,  -5,  -2, -20, -23, -44,
        -29, -51, -23, -15, -22, -18, -50, -64,
    ];

    #[rustfmt::skip]
    const MID_GAME_BISHOP_SQUARE_BONUS: [i16; 64] = [
        -29,   4, -82, -37, -25, -42,   7,  -8,
        -26,  16, -18, -13,  30,  59,  18, -47,
        -16,  37,  43,  40,  35,  50,  37,  -2,
         -4,   5,  19,  50,  37,  37,   7,  -2,
         -6,  13,  13,  26,  34,  12,  10,   4,
          0,  15,  15,  15,  14,  27,  18,  10,
          4,  15,  16,   0,   7,  21,  33,   1,
        -33,  -3, -14, -21, -13, -12, -39, -21,
    ];

    #[rustfmt::skip]
    const END_GAME_BISHOP_SQUARE_BONUS: [i16; 64] = [
        -14, -21, -11,  -8, -7,  -9, -17, -24,
         -8,  -4,   7, -12, -3, -13,  -4, -14,
          2,  -8,   0,  -1, -2,   6,   0,   4,
         -3,   9,  12,   9, 14,  10,   3,   2,
         -6,   3,  13,  19,  7,  10,  -3,  -9,
        -12,  -3,   8,  10, 13,   3,  -7, -15,
        -14, -18,  -7,  -1,  4,  -9, -15, -27,
        -23,  -9, -23,  -5, -9, -16,  -5, -17,
    ];

    #[rustfmt::skip]
    const MID_GAME_ROOK_SQUARE_BONUS: [i16; 64] = [
         32,  42,  32,  51, 63,  9,  31,  43,
         27,  32,  58,  62, 80, 67,  26,  44,
         -5,  19,  26,  36, 17, 45,  61,  16,
        -24, -11,   7,  26, 24, 35,  -8, -20,
        -36, -26, -12,  -1,  9, -7,   6, -23,
        -45, -25, -16, -17,  3,  0,  -5, -33,
        -44, -16, -20,  -9, -1, 11,  -6, -71,
        -19, -13,   1,  17, 16,  7, -37, -26,
    ];

    #[rustfmt::skip]
    const END_GAME_ROOK_SQUARE_BONUS: [i16; 64] = [
        13, 10, 18, 15, 12,  12,   8,   5,
        11, 13, 13, 11, -3,   3,   8,   3,
         7,  7,  7,  5,  4,  -3,  -5,  -3,
         4,  3, 13,  1,  2,   1,  -1,   2,
         3,  5,  8,  4, -5,  -6,  -8, -11,
        -4,  0, -5, -1, -7, -12,  -8, -16,
        -6, -6,  0,  2, -9,  -9, -11,  -3,
        -9,  2,  3, -1, -5, -13,   4, -20,
    ];

    #[rustfmt::skip]
    const MID_GAME_QUEEN_SQUARE_BONUS: [i16; 64] = [
        -28,   0,  29,  12,  59,  44,  43,  45,
        -24, -39,  -5,   1, -16,  57,  28,  54,
        -13, -17,   7,   8,  29,  56,  47,  57,
        -27, -27, -16, -16,  -1,  17,  -2,   1,
         -9, -26,  -9, -10,  -2,  -4,   3,  -3,
        -14,   2, -11,  -2,  -5,   2,  14,   5,
        -35,  -8,  11,   2,   8,  15,  -3,   1,
         -1, -18,  -9,  10, -15, -25, -31, -50,
    ];

    #[rustfmt::skip]
    const END_GAME_QUEEN_SQUARE_BONUS: [i16; 64] = [
         -9,  22,  22,  27,  27,  19,  10,  20,
        -17,  20,  32,  41,  58,  25,  30,   0,
        -20,   6,   9,  49,  47,  35,  19,   9,
          3,  22,  24,  45,  57,  40,  57,  36,
        -18,  28,  19,  47,  31,  34,  39,  23,
        -16, -27,  15,   6,   9,  17,  10,   5,
        -22, -23, -30, -16, -16, -23, -36, -32,
        -33, -28, -22, -43,  -5, -32, -20, -41,
    ];

    #[rustfmt::skip]
    const MID_GAME_KING_SQUARE_BONUS: [i16; 64] = [
        -65,  23,  16, -15, -56, -34,   2,  13,
         29,  -1, -20,  -7,  -8,  -4, -38, -29,
         -9,  24,   2, -16, -20,   6,  22, -22,
        -17, -20, -12, -27, -30, -25, -14, -36,
        -49,  -1, -27, -39, -46, -44, -33, -51,
        -14, -14, -22, -46, -44, -30, -15, -27,
          1,   7,  -8, -64, -43, -16,   9,   8,
        -15,  36,  12, -54,   8, -28,  24,  14,
    ];

    #[rustfmt::skip]
    const END_GAME_KING_SQUARE_BONUS: [i16; 64] = [
        -74, -35, -18, -18, -11,  15,   4, -17,
        -12,  17,  14,  17,  17,  38,  23,  11,
         10,  17,  23,  15,  20,  45,  44,  13,
         -8,  22,  24,  27,  26,  33,  26,   3,
        -18,  -4,  21,  24,  27,  23,   9, -11,
        -19,  -3,  11,  21,  23,  16,   7,  -9,
        -27, -11,   4,  13,  14,   4,  -5, -17,
        -53, -34, -21, -11, -28, -14, -24, -43
    ];

    const MID_GAME_SQUARE_BONUS: [[i16; 64]; 6] = [
        Self::MID_GAME_PAWN_SQUARE_BONUS,
        Self::MID_GAME_KNIGHT_SQUARE_BONUS,
        Self::MID_GAME_BISHOP_SQUARE_BONUS,
        Self::MID_GAME_ROOK_SQUARE_BONUS,
        Self::MID_GAME_QUEEN_SQUARE_BONUS,
        Self::MID_GAME_KING_SQUARE_BONUS,
    ];

    const END_GAME_SQUARE_BONUS: [[i16; 64]; 6] = [
        Self::END_GAME_PAWN_SQUARE_BONUS,
        Self::END_GAME_KNIGHT_SQUARE_BONUS,
        Self::END_GAME_BISHOP_SQUARE_BONUS,
        Self::END_GAME_ROOK_SQUARE_BONUS,
        Self::END_GAME_QUEEN_SQUARE_BONUS,
        Self::END_GAME_KING_SQUARE_BONUS,
    ];

    const PIECE_SQUARE_TABLE: [[[i16; 64]; 6]; 25] = Self::compute_piece_square_table();

    const fn compute_piece_square_table() -> [[[i16; 64]; 6]; 25] {
        let mut table = [[[0; 64]; 6]; 25];

        let mut phase = table.len();
        while phase > 0 {
            phase -= 1;

            let mut role = table[phase].len();
            while role > 0 {
                role -= 1;

                let mut square = table[phase][role].len();
                while square > 0 {
                    square -= 1;

                    let mg_score = Self::MID_GAME_PIECE_VALUE[role]
                        + Self::MID_GAME_SQUARE_BONUS[role][square];

                    let eg_score = Self::END_GAME_PIECE_VALUE[role]
                        + Self::END_GAME_SQUARE_BONUS[role][square];

                    table[phase][role][square] =
                        eg_score + (mg_score - eg_score) * phase as i16 / (table.len() as i16 - 1);
                }
            }
        }

        table
    }

    #[inline]
    fn piece_square_value(phase: usize, piece: Piece, square: Square) -> i16 {
        let p = phase.min(Self::PIECE_SQUARE_TABLE.len() - 1);
        let r: usize = piece.role() as _;
        let s: usize = match piece.color() {
            Color::White => square.mirror().index() as _,
            Color::Black => square.index() as _,
        };

        Self::PIECE_SQUARE_TABLE[p][r][s]
    }
}

impl Eval for Pesto {
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
                use Role::*;
                let phase = [Knight, Bishop, Rook, Queen]
                    .into_iter()
                    .map(|r| pos.by_role(r).len() * Self::GAME_PHASE_PIECE_WEIGHT[r as usize])
                    .sum();

                let mut score = [0; 2];

                use Color::*;
                for c in [White, Black] {
                    for r in [Pawn, Knight, Bishop, Rook, Queen, King] {
                        for s in pos.by_piece(Piece(c, r)) {
                            score[c as usize] += Self::piece_square_value(phase, Piece(c, r), s);
                        }
                    }
                }

                score[turn as usize] - score[!turn as usize]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Outcome;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(g: Game) {
        let engine = Pesto::new();
        assert_eq!(engine.eval(&g), engine.eval(&g.clone()));
    }

    #[proptest]
    fn draw_evaluates_to_balanced_score(
        #[filter(#_o.is_draw())] _o: Outcome,
        #[any(Some(#_o))] g: Game,
    ) {
        assert_eq!(Pesto::new().eval(&g), 0);
    }

    #[proptest]
    fn lost_game_evaluates_to_lowest_possible_score(
        #[filter(#_o.is_decisive())] _o: Outcome,
        #[any(Some(#_o))]
        #[filter(#_o.winner() != Some(#g.position().turn()))]
        g: Game,
    ) {
        assert_eq!(Pesto::new().eval(&g), i16::MIN);
    }

    #[proptest]
    fn won_game_evaluates_to_highest_possible_score(
        #[filter(#_o.is_decisive())] _o: Outcome,
        #[any(Some(#_o))]
        #[filter(#_o.winner() == Some(#g.position().turn()))]
        g: Game,
    ) {
        assert_eq!(Pesto::new().eval(&g), i16::MAX);
    }
}
