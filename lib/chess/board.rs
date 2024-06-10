use crate::{chess::*, util::Integer};
use arrayvec::ArrayString;
use derive_more::{Debug, Display, Error};
use std::fmt::{self, Write};
use std::{ops::Index, str::FromStr};

/// The chess board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("Board({self})")]
pub struct Board {
    #[cfg_attr(test, map(|mut bbs: [Bitboard; 6]| {
        let mut occupied = bbs[0];
        for bb in &mut bbs[1..] {
            *bb &= !occupied;
            occupied |= *bb;
        }

        bbs
    }))]
    roles: [Bitboard; 6],
    #[cfg_attr(test, map(|bb: Bitboard| {
        let occupied = #roles.iter().fold(Bitboard::empty(), |r, &b| r | b);
        [occupied & bb, occupied & !bb]
    }))]
    colors: [Bitboard; 2],
    pub turn: Color,
    pub castles: Castles,
    pub en_passant: Option<Square>,
    pub halfmoves: u8,
    pub fullmoves: u32,
}

impl Default for Board {
    #[inline(always)]
    fn default() -> Self {
        Self {
            roles: [
                Bitboard::new(0x00FF00000000FF00),
                Bitboard::new(0x4200000000000042),
                Bitboard::new(0x2400000000000024),
                Bitboard::new(0x8100000000000081),
                Bitboard::new(0x0800000000000008),
                Bitboard::new(0x1000000000000010),
            ],
            colors: [
                Bitboard::new(0x000000000000FFFF),
                Bitboard::new(0xFFFF000000000000),
            ],

            turn: Color::White,
            castles: Castles::all(),
            en_passant: None,
            halfmoves: 0,
            fullmoves: 1,
        }
    }
}

impl Board {
    /// [`Square`]s occupied by a [`Color`].
    #[inline(always)]
    pub fn by_color(&self, c: Color) -> Bitboard {
        self.colors[c as usize]
    }

    /// [`Square`]s occupied by a [`Role`].
    #[inline(always)]
    pub fn by_role(&self, r: Role) -> Bitboard {
        self.roles[r as usize]
    }

    /// [`Square`]s occupied by a [`Piece`].
    #[inline(always)]
    pub fn by_piece(&self, p: Piece) -> Bitboard {
        self.by_color(p.color()) & self.by_role(p.role())
    }

    /// [`Square`] occupied by a the king of a [`Color`].
    #[inline(always)]
    pub fn king(&self, side: Color) -> Option<Square> {
        let piece = Piece::new(Role::King, side);
        self.by_piece(piece).into_iter().next()
    }

    /// The [`Color`] of the piece on the given [`Square`], if any.
    #[inline(always)]
    pub fn color_on(&self, sq: Square) -> Option<Color> {
        Color::iter().find(|&c| self.by_color(c).contains(sq))
    }

    /// The [`Role`] of the piece on the given [`Square`], if any.
    #[inline(always)]
    pub fn role_on(&self, sq: Square) -> Option<Role> {
        Role::iter().find(|&r| self.by_role(r).contains(sq))
    }

    /// The [`Piece`] on the given [`Square`], if any.
    #[inline(always)]
    pub fn piece_on(&self, sq: Square) -> Option<Piece> {
        Option::zip(self.role_on(sq), self.color_on(sq)).map(|(r, c)| Piece::new(r, c))
    }

    /// An iterator over all pieces on the board.
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (Piece, Square)> + '_ {
        Piece::iter().flat_map(|p| self.by_piece(p).into_iter().map(move |sq| (p, sq)))
    }

    /// Computes the [zobrist hash].
    ///
    /// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
    #[inline(always)]
    pub fn zobrist(&self) -> Zobrist {
        let mut zobrist = ZobristNumbers::castling(self.castles);

        for (p, sq) in self.iter() {
            zobrist ^= ZobristNumbers::psq(p.color(), p.role(), sq);
        }

        if self.turn == Color::Black {
            zobrist ^= ZobristNumbers::turn();
        }

        if let Some(ep) = self.en_passant {
            zobrist ^= ZobristNumbers::en_passant(ep.file());
        }

        zobrist
    }

    /// Toggles a piece on a square.
    #[inline(always)]
    pub fn toggle(&mut self, p: Piece, sq: Square) {
        debug_assert!(!self[sq].is_some_and(|q| p != q));
        self.colors[p.color() as usize] ^= sq.bitboard();
        self.roles[p.role() as usize] ^= sq.bitboard();
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Board {
    type Output = Option<Piece>;

    #[inline(always)]
    fn index(&self, sq: Square) -> &Self::Output {
        match self.piece_on(sq) {
            Some(Piece::WhitePawn) => &Some(Piece::WhitePawn),
            Some(Piece::WhiteKnight) => &Some(Piece::WhiteKnight),
            Some(Piece::WhiteBishop) => &Some(Piece::WhiteBishop),
            Some(Piece::WhiteRook) => &Some(Piece::WhiteRook),
            Some(Piece::WhiteQueen) => &Some(Piece::WhiteQueen),
            Some(Piece::WhiteKing) => &Some(Piece::WhiteKing),
            Some(Piece::BlackPawn) => &Some(Piece::BlackPawn),
            Some(Piece::BlackKnight) => &Some(Piece::BlackKnight),
            Some(Piece::BlackBishop) => &Some(Piece::BlackBishop),
            Some(Piece::BlackRook) => &Some(Piece::BlackRook),
            Some(Piece::BlackQueen) => &Some(Piece::BlackQueen),
            Some(Piece::BlackKing) => &Some(Piece::BlackKing),
            None => &None,
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut skip = 0;
        for sq in Square::iter().map(|sq| sq.flip()) {
            let mut buffer = ArrayString::<2>::new();

            match self[sq] {
                None => skip += 1,
                Some(p) => write!(buffer, "{}", p)?,
            }

            if sq.file() == File::H {
                buffer.push(if sq.rank() == Rank::First { ' ' } else { '/' });
            }

            if !buffer.is_empty() && skip > 0 {
                write!(f, "{}", skip)?;
                skip = 0;
            }

            f.write_str(&buffer)?;
        }

        match self.turn {
            Color::White => f.write_str("w ")?,
            Color::Black => f.write_str("b ")?,
        }

        if self.castles != Castles::none() {
            write!(f, "{} ", self.castles)?;
        } else {
            f.write_str("- ")?;
        }

        if let Some(ep) = self.en_passant {
            write!(f, "{} ", ep)?;
        } else {
            f.write_str("- ")?;
        }

        write!(f, "{} {}", self.halfmoves, self.fullmoves)?;

        Ok(())
    }
}

/// The reason why parsing the FEN string failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
pub enum ParseFenError {
    #[display("failed to parse piece placement")]
    InvalidPlacement,
    #[display("failed to parse side to move")]
    InvalidSideToMove,
    #[display("failed to parse castling rights")]
    InvalidCastlingRights,
    #[display("failed to parse en passant square")]
    InvalidEnPassantSquare,
    #[display("failed to parse halfmove clock")]
    InvalidHalfmoveClock,
    #[display("failed to parse fullmove number")]
    InvalidFullmoveNumber,
    #[display("unspecified syntax error")]
    InvalidSyntax,
}

impl FromStr for Board {
    type Err = ParseFenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fields: Vec<_> = s.split(' ').collect();
        let [board, turn, castles, en_passant, halfmoves, fullmoves] = &fields[..] else {
            return Err(ParseFenError::InvalidSyntax);
        };

        let board: Vec<_> = board.split('/').rev().collect();
        let board @ [_1, _2, _3, _4, _5, _6, _7, _8] = &board[..] else {
            return Err(ParseFenError::InvalidPlacement);
        };

        let mut roles: [_; 6] = Default::default();
        let mut colors: [_; 2] = Default::default();
        for (rank, segment) in board.iter().enumerate() {
            let mut file = 0;
            for c in segment.chars() {
                let mut buffer = [0; 4];

                if file >= 8 {
                    return Err(ParseFenError::InvalidPlacement);
                } else if let Some(skip) = c.to_digit(10) {
                    file += skip;
                } else if let Ok(p) = Piece::from_str(c.encode_utf8(&mut buffer)) {
                    let sq = Square::new(File::new(file as _), Rank::new(rank as _));
                    colors[p.color() as usize] ^= sq.bitboard();
                    roles[p.role() as usize] ^= sq.bitboard();
                    file += 1;
                } else {
                    return Err(ParseFenError::InvalidPlacement);
                }
            }
        }

        let turn = match &turn[..] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(ParseFenError::InvalidSideToMove),
        };

        let castles = match &castles[..] {
            "-" => Castles::none(),
            _ => match castles.parse() {
                Err(_) => return Err(ParseFenError::InvalidCastlingRights),
                Ok(castles) => castles,
            },
        };

        let en_passant = match &en_passant[..] {
            "-" => None,
            ep => match ep.parse() {
                Err(_) => return Err(ParseFenError::InvalidEnPassantSquare),
                Ok(sq) => Some(sq),
            },
        };

        let Ok(halfmoves) = halfmoves.parse() else {
            return Err(ParseFenError::InvalidHalfmoveClock);
        };

        let Ok(fullmoves) = fullmoves.parse() else {
            return Err(ParseFenError::InvalidHalfmoveClock);
        };

        Ok(Board {
            roles,
            colors,
            turn,
            castles,
            en_passant,
            halfmoves,
            fullmoves,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
    use test_strategy::proptest;

    #[proptest]
    fn iter_returns_pieces_and_squares(b: Board) {
        for (p, sq) in b.iter() {
            assert_eq!(b[sq], Some(p));
        }
    }

    #[proptest]
    fn by_color_returns_squares_occupied_by_pieces_of_a_color(b: Board, c: Color) {
        for sq in b.by_color(c) {
            assert_eq!(b[sq].map(|p| p.color()), Some(c));
        }
    }

    #[proptest]
    fn by_color_returns_squares_occupied_by_pieces_of_a_role(b: Board, r: Role) {
        for sq in b.by_role(r) {
            assert_eq!(b[sq].map(|p| p.role()), Some(r));
        }
    }

    #[proptest]
    fn by_piece_returns_squares_occupied_by_a_piece(b: Board, p: Piece) {
        for sq in b.by_piece(p) {
            assert_eq!(b[sq], Some(p));
        }
    }

    #[proptest]
    fn king_returns_square_occupied_by_a_king(b: Board, c: Color) {
        if let Some(sq) = b.king(c) {
            assert_eq!(b[sq], Some(Piece::new(Role::King, c)));
        }
    }

    #[proptest]
    fn piece_on_returns_piece_on_the_given_square(b: Board, sq: Square) {
        assert_eq!(
            b.piece_on(sq),
            Option::zip(b.color_on(sq), b.role_on(sq)).map(|(c, r)| Piece::new(r, c))
        );
    }

    #[proptest]
    fn toggle_removes_piece_from_square(mut b: Board, #[filter(#b[#sq].is_some())] sq: Square) {
        let p = b[sq].unwrap();
        b.toggle(p, sq);
        assert_eq!(b[sq], None);
    }

    #[proptest]
    fn toggle_places_piece_on_square(
        mut b: Board,
        #[filter(#b[#sq].is_none())] sq: Square,
        p: Piece,
    ) {
        b.toggle(p, sq);
        assert_eq!(b[sq], Some(p));
    }

    #[proptest]
    #[should_panic]
    fn toggle_panics_if_square_occupied_by_other_piece(
        mut b: Board,
        #[filter(#b[#sq].is_some())] sq: Square,
        #[filter(Some(#p) != #b[#sq])] p: Piece,
    ) {
        b.toggle(p, sq);
    }

    #[proptest]
    fn board_can_be_indexed_by_square(b: Board, sq: Square) {
        assert_eq!(b[sq], b.piece_on(sq));
    }

    #[proptest]
    fn parsing_printed_board_is_an_identity(b: Board) {
        assert_eq!(b.to_string().parse(), Ok(b));
    }

    #[proptest]
    fn parsing_board_fails_for_invalid_fen(
        b: Board,
        #[strategy(..=#b.to_string().len())] n: usize,
        #[strategy("[^[:ascii:]]+")] r: String,
    ) {
        let s = b.to_string();
        assert_eq!([&s[..n], &r, &s[n..]].concat().parse().ok(), None::<Board>);
    }
}
