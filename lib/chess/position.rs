use crate::chess::{Bitboard, Color, Outcome, Piece, Promotion, Role, Square};
use crate::chess::{Move, MoveContext, MoveKind};
use crate::util::Bits;
use arrayvec::ArrayVec;
use derive_more::{DebugCustom, Display, Error, From};
use proptest::sample::{Selector, SelectorStrategy};
use proptest::{prelude::*, strategy::Map};
use shakmaty as sm;
use std::hash::{Hash, Hasher};
use std::ops::{Index, Range};
use std::{num::NonZeroU32, str::FromStr};
use test_strategy::Arbitrary;

/// A type representing a [`Position`]'s [zobrist hash].
///
/// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
pub type Zobrist = Bits<u64, 64>;

/// Represents an illegal [`Move`] in a given [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "move `{_0}` is illegal in this position")]
pub struct IllegalMove(#[error(not(source))] pub Move);

/// Represents an impossible [null-move] in a given [`Position`].
///
/// [null-move]: https://www.chessprogramming.org/Null_Move
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "passing the turn leads to illegal position")]
pub struct ImpossiblePass;

/// Represents an impossible exchange on a given [`Square`] in a given [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "no possible exchange on square `{_0}`")]
pub struct ImpossibleExchange(#[error(not(source))] pub Square);

/// The current position on the chess board.
///
/// This type guarantees that it only holds valid positions.
#[derive(DebugCustom, Display, Default, Clone, Eq)]
#[debug(fmt = "Position({self})")]
#[display(
    fmt = "{}",
    "sm::fen::Fen::from_position(self.0.clone(), sm::EnPassantMode::Legal)"
)]
pub struct Position(sm::Chess, [ArrayVec<Bits<u16, 16>, 51>; 2]);

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.zobrist().get())
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Arbitrary for Position {
    type Parameters = ();
    type Strategy = Map<(Range<usize>, SelectorStrategy), fn((usize, Selector)) -> Position>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        (0..256, any::<Selector>()).prop_map(|(moves, selector)| {
            use sm::{zobrist::*, *};
            let mut chess = Chess::default();
            let mut history = [ArrayVec::<_, 51>::new(), ArrayVec::<_, 51>::new()];

            for _ in 0..moves {
                match selector.try_select(chess.legal_moves()) {
                    None => break,
                    Some(m) => {
                        if m.is_zeroing() {
                            history = Default::default();
                        } else {
                            let history = &mut history[chess.turn() as usize];
                            let zobrist: Zobrist64 = chess.zobrist_hash(EnPassantMode::Legal);
                            if history.try_push(Zobrist::new(zobrist.0).pop()).is_err() {
                                break;
                            }
                        };

                        chess.play_unchecked(&m);
                    }
                }
            }

            Position(chess, history)
        })
    }
}

impl Position {
    /// The side to move.
    pub fn turn(&self) -> Color {
        sm::Position::turn(&self.0).into()
    }

    /// The number of halfmoves since the last capture or pawn advance.
    ///
    /// It resets to 0 whenever a piece is captured or a pawn is moved.
    pub fn halfmoves(&self) -> u32 {
        sm::Position::halfmoves(&self.0)
    }

    /// The current move number since the start of the game.
    ///
    /// It starts at 1, and is incremented after every move by black.
    pub fn fullmoves(&self) -> NonZeroU32 {
        sm::Position::fullmoves(&self.0)
    }

    /// The en passant square.
    pub fn en_passant_square(&self) -> Option<Square> {
        sm::Position::ep_square(&self.0, sm::EnPassantMode::Legal).map(Square::from)
    }

    /// This position's [zobrist hash].
    ///
    /// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
    pub fn zobrist(&self) -> Zobrist {
        let z: sm::zobrist::Zobrist64 =
            sm::zobrist::ZobristHash::zobrist_hash(&self.0, sm::EnPassantMode::Legal);
        Bits::new(z.0)
    }

    /// An iterator over all pieces on the board.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Piece, Square)> + ExactSizeIterator {
        sm::Position::board(&self.0)
            .clone()
            .into_iter()
            .map(|(s, p)| (p.into(), s.into()))
    }

    /// [`Square`]s occupied.
    pub fn occupied(&self) -> Bitboard {
        sm::Position::board(&self.0).occupied().into()
    }

    /// [`Square`]s occupied by a [`Color`].
    pub fn by_color(&self, c: Color) -> Bitboard {
        sm::Position::board(&self.0).by_color(c.into()).into()
    }

    /// [`Square`]s occupied by a [`Role`].
    pub fn by_role(&self, r: Role) -> Bitboard {
        sm::Position::board(&self.0).by_role(r.into()).into()
    }

    /// [`Square`]s occupied by a [`Piece`].
    pub fn by_piece(&self, p: Piece) -> Bitboard {
        sm::Position::board(&self.0).by_piece(p.into()).into()
    }

    /// [`Square`] occupied by a the king of the given color.
    pub fn king(&self, side: Color) -> Square {
        self.by_piece(Piece(side, Role::King))
            .into_iter()
            .next()
            .expect("expected king on the board")
    }

    /// The [`Role`] of the piece on the given [`Square`], if any.
    pub fn role_on(&self, s: Square) -> Option<Role> {
        sm::Position::board(&self.0)
            .role_at(s.into())
            .map(Role::from)
    }

    /// The [`Color`] of the piece on the given [`Square`], if any.
    pub fn color_on(&self, s: Square) -> Option<Color> {
        sm::Position::board(&self.0)
            .color_at(s.into())
            .map(Color::from)
    }

    /// The [`Piece`] on the given [`Square`], if any.
    pub fn piece_on(&self, s: Square) -> Option<Piece> {
        sm::Position::board(&self.0)
            .piece_at(s.into())
            .map(Piece::from)
    }

    /// Into where the piece in this [`Square`] can attack.
    pub fn attacks(&self, s: Square) -> Bitboard {
        sm::Position::board(&self.0).attacks_from(s.into()).into()
    }

    /// From where pieces of this [`Color`] can attack into this [`Square`].
    pub fn attackers(&self, s: Square, c: Color) -> Bitboard {
        let board = sm::Position::board(&self.0);
        board
            .attacks_to(s.into(), c.into(), board.occupied())
            .into()
    }

    /// How many other times this position has repeated.
    pub fn repetitions(&self) -> usize {
        let zobrist = self.zobrist().pop();
        let history = &self.1[self.turn() as usize];
        history.iter().filter(|z| **z == zobrist).count()
    }

    /// Whether this position is a [check].
    ///
    /// [check]: https://www.chessprogramming.org/Check
    pub fn is_check(&self) -> bool {
        sm::Position::is_check(&self.0)
    }

    /// Whether this position is a [checkmate].
    ///
    /// [checkmate]: https://www.chessprogramming.org/Checkmate
    pub fn is_checkmate(&self) -> bool {
        sm::Position::is_checkmate(&self.0)
    }

    /// Whether this position is a [stalemate].
    ///
    /// [stalemate]: https://www.chessprogramming.org/Stalemate
    pub fn is_stalemate(&self) -> bool {
        sm::Position::is_stalemate(&self.0)
    }

    /// Whether the game is a draw by [Threefold repetition].
    ///
    /// [Threefold repetition]: https://en.wikipedia.org/wiki/Threefold_repetition
    pub fn is_draw_by_threefold_repetition(&self) -> bool {
        self.repetitions() > 1
    }

    /// Whether the game is a draw by the [50-move rule].
    ///
    /// [50-move rule]: https://en.wikipedia.org/wiki/Fifty-move_rule
    pub fn is_draw_by_50_move_rule(&self) -> bool {
        self.halfmoves() >= 100
    }

    /// Whether this position has [insufficient material].
    ///
    /// [insufficient material]: https://www.chessprogramming.org/Material#InsufficientMaterial
    pub fn is_material_insufficient(&self) -> bool {
        sm::Position::is_insufficient_material(&self.0)
    }

    /// The [`Outcome`] of the game in case this position is final.
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

    /// An iterator over the legal [`Move`]s that can be played in this position.
    pub fn moves(
        &self,
        kind: MoveKind,
    ) -> impl DoubleEndedIterator<Item = MoveContext> + ExactSizeIterator {
        let mut legals = sm::Position::legal_moves(&self.0);
        legals.retain(|vm| kind.intersects(vm.into()));
        legals.into_iter().map(|vm| vm.into())
    }

    /// Play a [`Move`] if legal in this position.
    pub fn play(&mut self, m: Move) -> Result<MoveContext, IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.0) {
            Ok(vm) if sm::Position::is_legal(&self.0, &vm) => {
                if vm.is_zeroing() {
                    self.1 = Default::default();
                } else {
                    let zobrist = self.zobrist().pop();
                    let history = &mut self.1[self.turn() as usize];
                    if history.try_push(zobrist).is_err() {
                        return Err(IllegalMove(m));
                    }
                }

                sm::Position::play_unchecked(&mut self.0, &vm);
                Ok(vm.into())
            }

            _ => Err(IllegalMove(m)),
        }
    }

    /// Play a [null-move] if legal in this position.
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    pub fn pass(&mut self) -> Result<(), ImpossiblePass> {
        if self.is_check() {
            Err(ImpossiblePass)
        } else {
            let zobrist = self.zobrist().pop();
            let history = &mut self.1[self.turn() as usize];
            if history.try_push(zobrist).is_err() {
                return Err(ImpossiblePass);
            }

            let null = sm::Move::Put {
                role: sm::Role::King,
                to: self.king(self.turn()).into(),
            };

            sm::Position::play_unchecked(&mut self.0, &null);

            Ok(())
        }
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    pub fn exchange(&mut self, whither: Square) -> Result<MoveContext, ImpossibleExchange> {
        let capture = self.role_on(whither).ok_or(ImpossibleExchange(whither))?;

        let (whence, role) = self
            .attackers(whither, self.turn())
            .into_iter()
            .filter_map(|s| Some((s, self.role_on(s)?)))
            .min_by_key(|(_, r)| *r)
            .ok_or(ImpossibleExchange(whither))?;

        let promotion = match (role, whither.rank().index()) {
            (Role::Pawn, 0 | 7) => Promotion::Queen,
            _ => Promotion::None,
        };

        let vm = sm::Move::Normal {
            role: role.into(),
            from: whence.into(),
            capture: Some(capture.into()),
            to: whither.into(),
            promotion: promotion.into(),
        };

        if sm::Position::is_legal(&self.0, &vm) {
            self.1 = Default::default();
            sm::Position::play_unchecked(&mut self.0, &vm);
            Ok(vm.into())
        } else {
            Err(ImpossibleExchange(whither))
        }
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Position {
    type Output = Option<Piece>;

    fn index(&self, s: Square) -> &Self::Output {
        use {Color::*, Role::*};
        match self.piece_on(s) {
            Some(Piece(White, Pawn)) => &Some(Piece(White, Pawn)),
            Some(Piece(White, Knight)) => &Some(Piece(White, Knight)),
            Some(Piece(White, Bishop)) => &Some(Piece(White, Bishop)),
            Some(Piece(White, Rook)) => &Some(Piece(White, Rook)),
            Some(Piece(White, Queen)) => &Some(Piece(White, Queen)),
            Some(Piece(White, King)) => &Some(Piece(White, King)),
            Some(Piece(Black, Pawn)) => &Some(Piece(Black, Pawn)),
            Some(Piece(Black, Knight)) => &Some(Piece(Black, Knight)),
            Some(Piece(Black, Bishop)) => &Some(Piece(Black, Bishop)),
            Some(Piece(Black, Rook)) => &Some(Piece(Black, Rook)),
            Some(Piece(Black, Queen)) => &Some(Piece(Black, Queen)),
            Some(Piece(Black, King)) => &Some(Piece(Black, King)),
            None => &None,
        }
    }
}

/// The reason why parsing the FEN string failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
pub enum ParsePositionError {
    InvalidFen(InvalidFen),
    IllegalPosition(IllegalPosition),
}

/// The reason why the string is not valid FEN.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
pub enum InvalidFen {
    #[display(fmt = "syntax error at the piece placement field")]
    InvalidPlacement,
    #[display(fmt = "syntax error at the side to move field")]
    InvalidTurn,
    #[display(fmt = "syntax error at the castling rights field")]
    InvalidCastlingRights,
    #[display(fmt = "syntax error at the en passant square field")]
    InvalidEnPassantSquare,
    #[display(fmt = "syntax error at the halfmove clock field")]
    InvalidHalfmoveClock,
    #[display(fmt = "syntax error at the fullmove counter field")]
    InvalidFullmoves,
    #[display(fmt = "unspecified syntax error")]
    InvalidSyntax,
}

#[doc(hidden)]
impl From<sm::fen::ParseFenError> for InvalidFen {
    fn from(e: sm::fen::ParseFenError) -> Self {
        use InvalidFen::*;
        match e {
            sm::fen::ParseFenError::InvalidBoard => InvalidPlacement,
            sm::fen::ParseFenError::InvalidTurn => InvalidTurn,
            sm::fen::ParseFenError::InvalidCastling => InvalidCastlingRights,
            sm::fen::ParseFenError::InvalidEpSquare => InvalidEnPassantSquare,
            sm::fen::ParseFenError::InvalidHalfmoveClock => InvalidHalfmoveClock,
            sm::fen::ParseFenError::InvalidFullmoves => InvalidFullmoves,
            _ => InvalidSyntax,
        }
    }
}

/// The reason why the position represented by the FEN string is illegal.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
pub enum IllegalPosition {
    #[display(fmt = "at least one side has no king")]
    MissingKing,
    #[display(fmt = "at least one side has multiple kings")]
    TooManyKings,
    #[display(fmt = "there are pawns on the back-rank")]
    PawnsOnBackRank,
    #[display(fmt = "the player in check is not to move")]
    OppositeCheck,
    #[display(fmt = "invalid en passant square; wrong rank, occupied, or missing pushed pawn")]
    InvalidEnPassantSquare,
    #[display(fmt = "invalid castling rights")]
    InvalidCastlingRights,
    #[display(fmt = "no sequence of legal moves can reach this position")]
    Other,
}

#[doc(hidden)]
impl From<sm::PositionError<sm::Chess>> for IllegalPosition {
    fn from(e: sm::PositionError<sm::Chess>) -> Self {
        let kinds = e.kinds();

        if kinds.contains(sm::PositionErrorKinds::MISSING_KING) {
            IllegalPosition::MissingKing
        } else if kinds.contains(sm::PositionErrorKinds::TOO_MANY_KINGS) {
            IllegalPosition::TooManyKings
        } else if kinds.contains(sm::PositionErrorKinds::PAWNS_ON_BACKRANK) {
            IllegalPosition::PawnsOnBackRank
        } else if kinds.contains(sm::PositionErrorKinds::OPPOSITE_CHECK) {
            IllegalPosition::OppositeCheck
        } else if kinds.contains(sm::PositionErrorKinds::INVALID_EP_SQUARE) {
            IllegalPosition::InvalidEnPassantSquare
        } else if kinds.contains(sm::PositionErrorKinds::INVALID_CASTLING_RIGHTS) {
            IllegalPosition::InvalidCastlingRights
        } else {
            IllegalPosition::Other
        }
    }
}

impl FromStr for Position {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fen: sm::fen::Fen = s.parse().map_err(InvalidFen::from)?;
        let chess: sm::Chess = sm::Setup::from(fen)
            .position(sm::CastlingMode::Standard)
            .map_err(IllegalPosition::from)?;

        Ok(Position(chess, Default::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::Selector;
    use std::collections::HashSet;
    use test_strategy::proptest;

    #[proptest]
    fn turn_returns_the_current_side_to_play(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Legal);
        assert_eq!(pos.turn(), setup.turn.into());
    }

    #[proptest]
    fn halfmoves_returns_the_number_of_halfmoves_since_last_irreversible_move(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Legal);
        assert_eq!(pos.halfmoves(), setup.halfmoves);
    }

    #[proptest]
    fn fullmoves_returns_the_current_move_number(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Legal);
        assert_eq!(pos.fullmoves(), setup.fullmoves);
    }

    #[proptest]
    fn en_passant_square_returns_the_en_passant_square(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Legal);
        assert_eq!(pos.en_passant_square(), setup.ep_square.map(Square::from));
    }

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
        assert_eq!(pos[pos.king(c)], Some(Piece(c, Role::King)));
    }

    #[proptest]
    fn piece_on_returns_piece_on_the_given_square(pos: Position, s: Square) {
        assert_eq!(
            pos.piece_on(s),
            Option::zip(pos.color_on(s), pos.role_on(s)).map(|(c, r)| Piece(c, r))
        );
    }

    #[proptest]
    fn attacks_returns_squares_attacked_by_this_piece(pos: Position, s: Square) {
        for whither in pos.attacks(s) {
            assert!(pos.attackers(whither, pos[s].unwrap().color()).contains(s))
        }
    }

    #[proptest]
    fn attacks_returns_empty_iterator_if_square_is_not_occupied(
        pos: Position,
        #[filter(#pos[#s].is_none())] s: Square,
    ) {
        assert_eq!(pos.attacks(s).len(), 0);
    }

    #[proptest]
    fn attackers_returns_squares_from_where_pieces_of_a_color_can_attack(
        pos: Position,
        s: Square,
        c: Color,
    ) {
        for whence in pos.attackers(s, c) {
            assert!(pos.attacks(whence).contains(s))
        }
    }

    #[proptest]
    fn exchange_finds_attacker_of_least_value(pos: Position, s: Square) {
        match pos.clone().exchange(s) {
            Ok(m) => {
                let attackers = pos.attackers(s, pos.turn());
                assert!(attackers.contains(m.whence()));

                assert_eq!(
                    attackers.into_iter().filter_map(|a| pos.role_on(a)).min(),
                    Some(m.role()),
                );
            }

            Err(_) => {
                if pos[s].is_some() {
                    for a in pos.attackers(s, pos.turn()) {
                        let m = Move(a, s, Promotion::None);
                        assert_eq!(pos.clone().play(m), Err(IllegalMove(m)));

                        let m = Move(a, s, Promotion::Queen);
                        assert_eq!(pos.clone().play(m), Err(IllegalMove(m)));
                    }
                }
            }
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
    fn moves_returns_all_legal_moves_from_this_position(
        #[filter(#pos.outcome().is_none())] pos: Position,
    ) {
        for m in pos.moves(MoveKind::ANY) {
            let mut pos = pos.clone();
            assert_eq!(pos[m.whence()].map(|p| p.color()), Some(pos.turn()));
            assert_eq!(pos.play(*m), Ok(m));
        }
    }

    #[proptest]
    fn captures_reduce_material(pos: Position) {
        for m in pos.moves(MoveKind::CAPTURE) {
            let mut p = pos.clone();
            p.play(*m)?;
            assert!(p.by_color(p.turn()).len() < pos.by_color(p.turn()).len());
        }
    }

    #[proptest]
    fn promotions_exchange_pawns(pos: Position) {
        for m in pos.moves(MoveKind::PROMOTION) {
            let mut p = pos.clone();
            p.play(*m)?;
            let pawn = Piece(pos.turn(), Role::Pawn);
            assert!(p.by_piece(pawn).len() < pos.by_piece(pawn).len());
            assert_eq!(p.by_color(pos.turn()).len(), pos.by_color(pos.turn()).len());
        }
    }

    #[proptest]
    fn castles_move_the_king_by_two_files(pos: Position) {
        for m in pos.moves(MoveKind::CASTLE) {
            assert_eq!(pos[m.whence()], Some(Piece(pos.turn(), Role::King)));
            assert_eq!(m.whence().rank(), m.whither().rank());
            assert_eq!((m.whence().file() - m.whither().file()).abs(), 2);
        }
    }

    #[proptest]
    fn castles_are_neither_captures_nor_promotions(pos: Position) {
        let castles: HashSet<_> = pos.moves(MoveKind::CASTLE).collect();
        let captures_or_promotions: HashSet<_> =
            pos.moves(MoveKind::CAPTURE | MoveKind::PROMOTION).collect();

        assert_eq!(castles.intersection(&captures_or_promotions).count(), 0);
    }

    #[proptest]
    fn legal_move_updates_position(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        selector: Selector,
    ) {
        let m = selector.select(pos.moves(MoveKind::ANY));
        let prev = pos.clone();
        assert_eq!(pos.play(*m), Ok(m));
        assert_ne!(pos, prev);
    }

    #[proptest]
    fn illegal_move_fails_without_changing_position(
        mut pos: Position,
        #[filter(#pos.clone().play(#m).is_err())] m: Move,
    ) {
        let before = pos.clone();
        assert_eq!(pos.play(m), Err(IllegalMove(m)));
        assert_eq!(pos, before);
    }

    #[proptest]
    fn pass_updates_position(#[filter(#pos.clone().pass().is_ok())] mut pos: Position) {
        let before = pos.clone();
        assert_eq!(pos.pass(), Ok(()));
        assert_ne!(pos, before);
    }

    #[proptest]
    fn impossible_pass_preserves_position(
        #[filter(#pos.clone().pass().is_err())] mut pos: Position,
    ) {
        let before = pos.clone();
        assert_eq!(pos.pass(), Err(ImpossiblePass));
        assert_eq!(pos, before);
    }

    #[proptest]
    fn threefold_repetition_implies_draw(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        selector: Selector,
    ) {
        let rep = pos.clone();
        let m = selector.select(pos.moves(MoveKind::ANY));
        let n = Move(m.whither(), m.whence(), Promotion::None);

        for _ in 0..2 {
            prop_assume!(pos.play(*m).is_ok());
            prop_assume!(pos.pass().is_ok());
            prop_assume!(pos.play(n).is_ok());
            prop_assume!(pos.pass().is_ok());
            prop_assume!(pos == rep);
        }

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
    fn parsing_invalid_fen_fails(
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
