use crate::chess::*;
use crate::util::{Assume, Bits, Integer};
use arrayvec::ArrayVec;
use cozy_chess as cc;
use derive_more::{Debug, Display, Error};
use std::hash::{Hash, Hasher};
use std::{num::NonZeroU32, ops::Index, str::FromStr};

#[cfg(test)]
use proptest::{prelude::*, sample::*};

/// A type representing a [`Position`]'s [zobrist hash].
///
/// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
pub type Zobrist = Bits<u64, 64>;

/// Represents an impossible exchange on a given [`Square`] in a given [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display("no possible exchange on square `{_0}`")]
pub struct ImpossibleExchange(#[error(not(source))] pub Square);

/// The current position on the board board.
///
/// This type guarantees that it only holds valid positions.
#[derive(Debug, Display, Default, Clone)]
#[debug("Position({self})")]
#[display("{board}")]
pub struct Position {
    board: cc::Board,
    history: [[Option<NonZeroU32>; 8]; 2],
}

impl Hash for Position {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.zobrist().hash(state)
    }
}

impl Eq for Position {}

impl PartialEq for Position {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.board.eq(&other.board)
    }
}

#[cfg(test)]
impl Arbitrary for Position {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        (0..256, any::<Selector>())
            .prop_map(|(moves, selector)| {
                let mut board = cc::Board::default();
                let mut history: [[_; 8]; 2] = Default::default();

                for _ in 0..moves {
                    if board.halfmove_clock() >= 100 {
                        break;
                    }

                    let turn = board.side_to_move();
                    let mut moves = ArrayVec::<_, 18>::new();
                    board.generate_moves(|ms| {
                        moves.push(ms);
                        false
                    });

                    match selector.try_select(moves.into_iter().flatten()) {
                        None => break,
                        Some(m) => {
                            let zobrist = Zobrist::new(board.hash());
                            board.play_unchecked(m);
                            if board.halfmove_clock() > 0 {
                                let entries = history[turn as usize].len();
                                history[turn as usize].copy_within(..entries - 1, 1);
                                history[turn as usize][0] = NonZeroU32::new(zobrist.get() as _);
                            } else {
                                history = Default::default();
                            }
                        }
                    }
                }

                Position { board, history }
            })
            .no_shrink()
            .boxed()
    }
}

impl Position {
    /// The side to move.
    #[inline(always)]
    pub fn turn(&self) -> Color {
        self.board.side_to_move().into()
    }

    /// The number of halfmoves since the last capture or pawn advance.
    ///
    /// It resets to 0 whenever a piece is captured or a pawn is moved.
    #[inline(always)]
    pub fn halfmoves(&self) -> u8 {
        self.board.halfmove_clock()
    }

    /// The current move number since the start of the game.
    ///
    /// It starts at 1, and is incremented after every move by black.
    #[inline(always)]
    pub fn fullmoves(&self) -> NonZeroU32 {
        NonZeroU32::new(self.board.fullmove_number() as _).assume()
    }

    /// The en passant square.
    #[inline(always)]
    pub fn en_passant(&self) -> Option<Square> {
        self.board
            .en_passant()
            .map(|f| Square::new(f.into(), Rank::Sixth).perspective(self.turn()))
    }

    /// This position's [zobrist hash].
    ///
    /// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
    #[inline(always)]
    pub fn zobrist(&self) -> Zobrist {
        Bits::new(self.board.hash())
    }

    /// An iterator over all pieces on the board.
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = (Piece, Square)> + '_ {
        Piece::iter().flat_map(|p| self.by_piece(p).into_iter().map(move |s| (p, s)))
    }

    /// [`Square`]s occupied.
    #[inline(always)]
    pub fn occupied(&self) -> Bitboard {
        self.board.occupied().into()
    }

    /// [`Square`]s occupied by a [`Color`].
    #[inline(always)]
    pub fn by_color(&self, c: Color) -> Bitboard {
        self.board.colors(c.into()).into()
    }

    /// [`Square`]s occupied by a [`Role`].
    #[inline(always)]
    pub fn by_role(&self, r: Role) -> Bitboard {
        self.board.pieces(r.into()).into()
    }

    /// [`Square`]s occupied by a [`Piece`].
    #[inline(always)]
    pub fn by_piece(&self, p: Piece) -> Bitboard {
        self.board
            .colored_pieces(p.color().into(), p.role().into())
            .into()
    }

    /// [`Square`] occupied by a the king of the given color.
    #[inline(always)]
    pub fn king(&self, side: Color) -> Square {
        self.board.king(side.into()).into()
    }

    /// The [`Role`] of the piece on the given [`Square`], if any.
    #[inline(always)]
    pub fn role_on(&self, s: Square) -> Option<Role> {
        self.board.piece_on(s.into()).map(Role::from)
    }

    /// The [`Color`] of the piece on the given [`Square`], if any.
    #[inline(always)]
    pub fn color_on(&self, s: Square) -> Option<Color> {
        self.board.color_on(s.into()).map(Color::from)
    }

    /// The [`Piece`] on the given [`Square`], if any.
    #[inline(always)]
    pub fn piece_on(&self, s: Square) -> Option<Piece> {
        Option::zip(self.role_on(s), self.color_on(s)).map(|(r, c)| Piece::new(r, c))
    }

    /// Into where a [`Piece`] on this [`Square`] can attack.
    #[inline(always)]
    pub fn attacks(&self, s: Square, p: Piece) -> Bitboard {
        let blk = self.occupied().into();

        Bitboard::from(match p.role() {
            Role::Pawn => cc::get_pawn_attacks(s.into(), p.color().into()),
            Role::Knight => cc::get_knight_moves(s.into()),
            Role::Bishop => cc::get_bishop_moves(s.into(), blk),
            Role::Rook => cc::get_rook_moves(s.into(), blk),
            Role::Queen => cc::get_bishop_moves(s.into(), blk) | cc::get_rook_moves(s.into(), blk),
            Role::King => cc::get_king_moves(s.into()),
        })
    }

    /// From where a [`Piece`] can attack into this [`Square`].
    #[inline(always)]
    pub fn attackers(&self, s: Square, p: Piece) -> Bitboard {
        self.attacks(s, p.flip())
    }

    /// How many other times this position has repeated.
    #[inline(always)]
    pub fn repetitions(&self) -> usize {
        match NonZeroU32::new(self.zobrist().get() as _) {
            None => 0,
            hash => {
                let history = self.history[self.turn() as usize];
                history.iter().filter(|h| **h == hash).count()
            }
        }
    }

    /// Whether this position is a [check].
    ///
    /// [check]: https://www.chessprogramming.org/Check
    #[inline(always)]
    pub fn is_check(&self) -> bool {
        !self.board.checkers().is_empty()
    }

    /// Whether this position is a [checkmate].
    ///
    /// [checkmate]: https://www.chessprogramming.org/Checkmate
    #[inline(always)]
    pub fn is_checkmate(&self) -> bool {
        self.is_check() & !self.board.generate_moves(|_| true)
    }

    /// Whether this position is a [stalemate].
    ///
    /// [stalemate]: https://www.chessprogramming.org/Stalemate
    #[inline(always)]
    pub fn is_stalemate(&self) -> bool {
        !self.is_check() & !self.board.generate_moves(|_| true)
    }

    /// Whether the game is a draw by [Threefold repetition].
    ///
    /// [Threefold repetition]: https://en.wikipedia.org/wiki/Threefold_repetition
    #[inline(always)]
    pub fn is_draw_by_threefold_repetition(&self) -> bool {
        self.repetitions() > 1
    }

    /// Whether the game is a draw by the [50-move rule].
    ///
    /// [50-move rule]: https://en.wikipedia.org/wiki/Fifty-move_rule
    #[inline(always)]
    pub fn is_draw_by_50_move_rule(&self) -> bool {
        self.halfmoves() >= 100
    }

    /// Whether this position has [insufficient material].
    ///
    /// [insufficient material]: https://www.chessprogramming.org/Material#InsufficientMaterial
    #[inline(always)]
    pub fn is_material_insufficient(&self) -> bool {
        match self.occupied().len() {
            2 => true,
            3 => !self.by_role(Role::Bishop).is_empty() || !self.by_role(Role::Knight).is_empty(),
            _ => {
                let bishops = self.by_role(Role::Bishop);
                bishops | self.by_role(Role::King) == self.occupied()
                    && (bishops & Bitboard::light() == Bitboard::empty()
                        || bishops & Bitboard::dark() == Bitboard::empty())
            }
        }
    }

    /// The [`Outcome`] of the game in case this position is final.
    #[inline(always)]
    pub fn outcome(&self) -> Option<Outcome> {
        if self.is_checkmate() {
            Some(Outcome::Checkmate(!self.turn()))
        } else if self.is_stalemate() {
            Some(Outcome::Stalemate)
        } else if self.is_draw_by_threefold_repetition() {
            Some(Outcome::DrawByThreefoldRepetition)
        } else if self.is_draw_by_50_move_rule() {
            Some(Outcome::DrawBy50MoveRule)
        } else if self.is_material_insufficient() {
            Some(Outcome::DrawByInsufficientMaterial)
        } else {
            None
        }
    }

    /// An iterator over the legal [`Move`]s that can be played from a subset of squares in this position.
    #[inline(always)]
    pub fn moves(&self) -> impl Iterator<Item = Move> + '_ {
        let mut moves = ArrayVec::<_, 18>::new();
        self.board.generate_moves(|ms| {
            moves.push(ms);
            false
        });

        moves.into_iter().flat_map(move |ms| {
            let role = ms.piece.into();
            let whence = ms.from.into();
            ms.into_iter().map(move |m| {
                let whither = m.to.into();

                match role {
                    Role::Pawn if self.en_passant() == Some(whither) => {
                        Move::en_passant(whence, whither)
                    }

                    Role::King if whither.file() - whence.file() > 1 => {
                        Move::castling(whence, Square::new(File::G, whither.rank()))
                    }

                    Role::King if whence.file() - whither.file() > 1 => {
                        Move::castling(whence, Square::new(File::C, whither.rank()))
                    }

                    _ => {
                        let promotion = m.promotion.map(|r| r.into());

                        if self.role_on(whither).is_some() {
                            Move::capture(whence, whither, promotion)
                        } else {
                            Move::regular(whence, whither, promotion)
                        }
                    }
                }
            })
        })
    }

    /// Play a [`Move`].
    #[inline(always)]
    pub fn play(&mut self, m: Move) {
        let from = m.whence();
        let to = if !m.is_castling() {
            m.whither()
        } else if from < m.whither() {
            Square::new(File::H, m.whither().rank())
        } else {
            Square::new(File::A, m.whither().rank())
        };

        let m = cc::Move {
            from: from.into(),
            to: to.into(),
            promotion: m.promotion().map(Role::into),
        };

        let turn = self.turn();
        let zobrist = self.zobrist();

        debug_assert!(self.board.is_legal(m), "`{m}` is illegal in `{self}`");
        self.board.play_unchecked(m);

        if self.halfmoves() > 0 {
            let entries = self.history[turn as usize].len();
            self.history[turn as usize].copy_within(..entries - 1, 1);
            self.history[turn as usize][0] = NonZeroU32::new(zobrist.get() as _);
        } else {
            self.history = Default::default();
        }
    }

    /// Play a [null-move].
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    #[inline(always)]
    pub fn pass(&mut self) {
        debug_assert!(!self.is_check(), "null move is illegal in `{self}`");
        self.board = self.board.null_move().assume();
        self.history = Default::default();
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    #[inline(always)]
    pub fn exchange(&mut self, whither: Square) -> Result<Move, ImpossibleExchange> {
        let turn = self.turn();
        if !self.by_color(!turn).contains(whither) {
            return Err(ImpossibleExchange(whither));
        }

        for role in Role::iter() {
            let piece = Piece::new(role, turn);
            for whence in self.by_piece(piece) & self.attackers(whither, piece) {
                let ms = cc::PieceMoves {
                    piece: role.into(),
                    from: whence.into(),
                    to: cc::Square::from(whither).into(),
                };

                if let Some(m) = ms.into_iter().max_by_key(|m| m.promotion) {
                    if self.board.is_legal(m) {
                        self.board.play_unchecked(m);
                        self.history = Default::default();
                        let promotion = m.promotion.map(Role::from);
                        return Ok(Move::capture(whence, whither, promotion));
                    }
                }
            }
        }

        Err(ImpossibleExchange(whither))
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Position {
    type Output = Option<Piece>;

    #[inline(always)]
    fn index(&self, s: Square) -> &Self::Output {
        match self.piece_on(s) {
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

/// The reason why parsing the FEN string failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
pub enum ParsePositionError {
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

#[doc(hidden)]
impl From<cc::FenParseError> for ParsePositionError {
    fn from(e: cc::FenParseError) -> Self {
        use ParsePositionError::*;
        match e {
            cc::FenParseError::InvalidBoard => InvalidPlacement,
            cc::FenParseError::InvalidSideToMove => InvalidSideToMove,
            cc::FenParseError::InvalidCastlingRights => InvalidCastlingRights,
            cc::FenParseError::InvalidEnPassant => InvalidEnPassantSquare,
            cc::FenParseError::InvalidHalfMoveClock => InvalidHalfmoveClock,
            cc::FenParseError::InvalidFullmoveNumber => InvalidFullmoveNumber,
            cc::FenParseError::MissingField => InvalidSyntax,
            cc::FenParseError::TooManyFields => InvalidSyntax,
        }
    }
}

impl FromStr for Position {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Position {
            board: s.parse()?,
            history: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cmp::Reverse, fmt::Debug};
    use test_strategy::proptest;

    #[proptest]
    fn iter_returns_pieces_and_squares(pos: Position) {
        for (p, s) in pos.iter() {
            assert_eq!(pos[s], Some(p));
        }
    }

    #[proptest]
    fn occupied_returns_non_empty_squares(pos: Position) {
        for s in pos.occupied() {
            assert_ne!(pos[s], None);
        }
    }

    #[proptest]
    fn by_color_returns_squares_occupied_by_pieces_of_a_color(pos: Position, c: Color) {
        for s in pos.by_color(c) {
            assert_eq!(pos[s].map(|p| p.color()), Some(c));
        }
    }

    #[proptest]
    fn by_color_returns_squares_occupied_by_pieces_of_a_role(pos: Position, r: Role) {
        for s in pos.by_role(r) {
            assert_eq!(pos[s].map(|p| p.role()), Some(r));
        }
    }

    #[proptest]
    fn by_piece_returns_squares_occupied_by_a_piece(pos: Position, p: Piece) {
        for s in pos.by_piece(p) {
            assert_eq!(pos[s], Some(p));
        }
    }

    #[proptest]
    fn king_returns_square_occupied_by_a_king(pos: Position, c: Color) {
        assert_eq!(pos[pos.king(c)], Some(Piece::new(Role::King, c)));
    }

    #[proptest]
    fn piece_on_returns_piece_on_the_given_square(pos: Position, s: Square) {
        assert_eq!(
            pos.piece_on(s),
            Option::zip(pos.color_on(s), pos.role_on(s)).map(|(c, r)| Piece::new(r, c))
        );
    }

    #[proptest]
    fn attacks_returns_squares_attacked_by_a_piece(pos: Position) {
        for (p, s) in pos.iter() {
            for whither in pos.attacks(s, p) {
                assert!(pos.attackers(whither, p).contains(s))
            }
        }
    }

    #[proptest]
    fn attackers_returns_squares_from_where_a_piece_can_attack(pos: Position, s: Square, p: Piece) {
        for whence in pos.attackers(s, p) {
            assert!(pos.attacks(whence, p).contains(s))
        }
    }

    #[proptest]
    fn checkmate_implies_outcome(pos: Position) {
        assert!(!pos.is_checkmate() || pos.outcome() == Some(Outcome::Checkmate(!pos.turn())));
    }

    #[proptest]
    fn stalemate_implies_outcome(pos: Position) {
        assert!(!pos.is_stalemate() || pos.outcome() == Some(Outcome::Stalemate));
    }

    #[proptest]
    fn checkmate_implies_check(pos: Position) {
        assert!(!pos.is_checkmate() || pos.is_check());
    }

    #[proptest]
    fn checkmate_and_stalemate_are_mutually_exclusive(pos: Position) {
        assert!(!(pos.is_checkmate() && pos.is_stalemate()));
    }

    #[proptest]
    fn check_and_stalemate_are_mutually_exclusive(pos: Position) {
        assert!(!(pos.is_check() && pos.is_stalemate()));
    }

    #[proptest]
    fn moves_returns_legal_moves_from_this_position(
        #[filter(#pos.outcome().is_none())] pos: Position,
    ) {
        for m in pos.moves() {
            pos.clone().play(m);
        }
    }

    #[proptest]
    fn exchange_finds_attacker_of_least_value(
        #[by_ref]
        #[filter(#pos.moves().filter(|m| m.is_capture() && !m.is_en_passant()).next().is_some())]
        pos: Position,
        #[map(|s: Selector| s.select(#pos.moves().filter(|m| m.is_capture() && !m.is_en_passant())).whither())]
        s: Square,
    ) {
        let m = pos.clone().exchange(s)?;

        let lva = pos
            .moves()
            .filter(|m| m.whither() == s)
            .filter(Move::is_capture)
            .min_by_key(|m| (pos.role_on(m.whence()), Reverse(m.promotion())))
            .unwrap();

        assert_eq!(m.promotion(), lva.promotion());
        assert_eq!(pos.role_on(m.whence()), pos.role_on(lva.whence()));
    }

    #[proptest]
    fn exchange_fails_if_not_a_regular_capture(
        mut pos: Position,
        #[filter(#pos.color_on(#s) != Some(!#pos.turn()))] s: Square,
    ) {
        assert_eq!(pos.exchange(s), Err(ImpossibleExchange(s)));
    }

    #[proptest]
    fn captures_reduce_material(
        #[by_ref]
        #[filter(#pos.moves().filter(Move::is_capture).next().is_some())]
        pos: Position,
        #[map(|s: Selector| s.select(#pos.moves().filter(Move::is_capture)))] m: Move,
    ) {
        let mut p = pos.clone();
        p.play(m);
        assert!(p.by_color(p.turn()).len() < pos.by_color(p.turn()).len());
    }

    #[proptest]
    fn promotions_exchange_pawns(
        #[by_ref]
        #[filter(#pos.moves().filter(Move::is_promotion).next().is_some())]
        pos: Position,
        #[map(|s: Selector| s.select(#pos.moves().filter(Move::is_promotion)))] m: Move,
    ) {
        let mut p = pos.clone();
        p.play(m);
        let pawn = Piece::new(Role::Pawn, pos.turn());
        assert!(p.by_piece(pawn).len() < pos.by_piece(pawn).len());
        assert_eq!(p.by_color(pos.turn()).len(), pos.by_color(pos.turn()).len());
    }

    #[proptest]
    fn castles_move_the_king_by_two_files(
        #[by_ref]
        #[filter(#pos.moves().filter(Move::is_castling).next().is_some())]
        pos: Position,
        #[map(|s: Selector| s.select(#pos.moves().filter(Move::is_castling)))] m: Move,
    ) {
        assert_eq!(pos[m.whence()], Some(Piece::new(Role::King, pos.turn())));
        assert_eq!(m.whence().rank(), m.whither().rank());
        assert_eq!((m.whence().file() - m.whither().file()).abs(), 2);
    }

    #[proptest]
    fn legal_move_updates_position(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        let prev = pos.clone();
        pos.play(m);
        assert_ne!(pos, prev);
    }

    #[proptest]
    #[should_panic]
    fn play_panics_if_move_illegal(
        #[by_ref] mut pos: Position,
        #[filter(!#pos.moves().any(|m| (m.whence(), m.whither()) == (#m.whence(), #m.whither())))]
        m: Move,
    ) {
        pos.play(m);
    }

    #[proptest]
    fn pass_updates_position(#[filter(!#pos.is_check())] mut pos: Position) {
        let before = pos.clone();
        pos.pass();
        assert_ne!(pos, before);
    }

    #[proptest]
    #[should_panic]
    fn pass_panics_if_in_check(#[filter(#pos.is_check())] mut pos: Position) {
        pos.pass();
    }

    #[proptest]
    fn threefold_repetition_implies_draw(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        z: NonZeroU32,
    ) {
        let zobrist = NonZeroU32::new(pos.zobrist().get() as _);
        let history = [zobrist, Some(z), zobrist, Some(z)];
        pos.history[pos.turn() as usize][..4].clone_from_slice(&history);
        assert!(pos.is_draw_by_threefold_repetition());
        assert_eq!(pos.outcome(), Some(Outcome::DrawByThreefoldRepetition));
    }

    #[proptest]
    fn position_can_be_indexed_by_square(pos: Position, s: Square) {
        assert_eq!(pos[s], pos.piece_on(s));
    }

    #[proptest]
    fn parsing_printed_position_is_an_identity(pos: Position) {
        assert_eq!(pos.to_string().parse(), Ok(pos));
    }

    #[proptest]
    fn parsing_position_fails_for_invalid_fen(
        pos: Position,
        #[strategy(..=#pos.to_string().len())] n: usize,
        #[strategy("[^[:ascii:]]+")] r: String,
    ) {
        assert!([&pos.to_string()[..n], &r]
            .concat()
            .parse::<Position>()
            .is_err());
    }
}
