use super::{Color, Fen, Move, Piece, Role, San, Square};
use crate::util::Bits;
use bitflags::bitflags;
use bitvec::{order::Lsb0, view::BitView};
use derive_more::{DebugCustom, Display, Error};
use proptest::{prelude::*, sample::Selector};
use shakmaty as sm;
use std::{convert::TryFrom, num::NonZeroU32, ops::Index};
use test_strategy::Arbitrary;

bitflags! {
    /// Characteristics of a [`Move`] in the context of a [`Position`].
    #[derive(Default)]
    pub struct MoveKind: u8 {
        const ANY =         0b00000001;
        const CASTLE =      0b00000010;
        const PROMOTION =   0b00000100;
        const CAPTURE =     0b00001000;
    }
}

#[doc(hidden)]
impl From<&sm::Move> for MoveKind {
    fn from(m: &sm::Move) -> Self {
        let mut kind = Self::ANY;

        if m.is_castle() {
            kind |= MoveKind::CASTLE
        }

        if m.is_promotion() {
            kind |= MoveKind::PROMOTION
        }

        if m.is_capture() {
            kind |= MoveKind::CAPTURE;
        }

        kind
    }
}

#[doc(hidden)]
impl From<&mut sm::Move> for MoveKind {
    fn from(m: &mut sm::Move) -> Self {
        (&*m).into()
    }
}

#[doc(hidden)]
impl From<sm::Move> for MoveKind {
    fn from(m: sm::Move) -> Self {
        (&m).into()
    }
}

pub type Zobrist = Bits<u64, 64>;

/// Represents an illegal [`Move`] in a given [`Position`].
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "move `{}` is illegal in position `{}`", _0, _1)]
pub struct IllegalMove(pub Move, pub Position);

/// Represents an illegal [null-move] in a given [`Position`].
///
/// [null-move]: https://www.chessprogramming.org/Null_Move
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "passing the turn leads to illegal position from `{}`", _0)]
pub struct IllegalPass(#[error(not(source))] pub Position);

/// The current position on the chess board.
///
/// This type guarantees that it only holds valid positions.
#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[debug(fmt = "Position(\"{}\")", self)]
#[display(fmt = "{}", "Fen::from(self.clone())")]
pub struct Position(
    #[strategy((0..256, any::<Selector>()).prop_map(|(moves, selector)| {
        let mut chess = sm::Chess::default();
        for _ in 0..moves {
            match selector.try_select(sm::Position::legal_moves(&chess)) {
                Some(m) => sm::Position::play_unchecked(&mut chess, &m),
                _ => break,
            }
        }
        chess
    }).no_shrink())]
    sm::Chess,
);

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

    /// This position's [zobrist hash].
    ///
    /// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
    pub fn zobrist(&self) -> Zobrist {
        sm::zobrist::ZobristHash::zobrist_hash::<u64>(&self.0)
            .view_bits::<Lsb0>()
            .into()
    }

    /// [`Square`]s occupied.
    pub fn occupied(&self) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.0)
            .occupied()
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`]s occupied by a [`Color`].
    pub fn by_color(&self, c: Color) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.0)
            .by_color(c.into())
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`]s occupied by a [`Role`].
    pub fn by_role(&self, r: Role) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.0)
            .by_role(r.into())
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`]s occupied by a [`Piece`].
    pub fn by_piece(&self, p: Piece) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.0)
            .by_piece(p.into())
            .into_iter()
            .map(Square::from)
    }

    /// Into where the piece in this [`Square`] can attack.
    pub fn attacks(&self, s: Square) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.0)
            .attacks_from(s.into())
            .into_iter()
            .map(Square::from)
    }

    /// From where pieces of this [`Color`] can attack into this [`Square`].
    pub fn attackers(&self, s: Square, c: Color) -> impl ExactSizeIterator<Item = Square> {
        let board = sm::Position::board(&self.0);
        board
            .attacks_to(s.into(), c.into(), board.occupied())
            .into_iter()
            .map(Square::from)
    }

    /// The [`Square`]s occupied by [`Piece`]s giving check.
    pub fn checkers(&self) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::checkers(&self.0)
            .into_iter()
            .map(Square::from)
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

    /// Whether this position has [insufficient material].
    ///
    /// [insufficient material]: https://www.chessprogramming.org/Material#InsufficientMaterial
    pub fn is_material_insufficient(&self) -> bool {
        sm::Position::is_insufficient_material(&self.0)
    }

    /// An iterator over the legal [`Move`]s that can be played in this position.
    pub fn moves(&self, kind: MoveKind) -> impl ExactSizeIterator<Item = (Move, MoveKind, Self)> {
        let mut legals = sm::Position::legal_moves(&self.0);
        legals.retain(|vm| kind.intersects(vm.into()));

        let p = self.0.clone();
        legals.into_iter().map(move |vm| {
            let mut p = p.clone();
            sm::Position::play_unchecked(&mut p, &vm);
            (sm::uci::Uci::from_standard(&vm).into(), vm.into(), p.into())
        })
    }

    /// Play a [`Move`] if legal in this position.
    pub fn make(&mut self, m: Move) -> Result<San, IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.0) {
            Ok(vm) if sm::Position::is_legal(&self.0, &vm) => {
                let san = sm::san::San::from_move(&self.0, &vm).into();
                sm::Position::play_unchecked(&mut self.0, &vm);
                Ok(san)
            }

            _ => Err(IllegalMove(m, self.clone())),
        }
    }

    /// Play a [null-move] if legal in this position.
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    pub fn pass(&mut self) -> Result<San, IllegalPass> {
        match sm::Position::swap_turn(self.0.clone()) {
            Err(_) => Err(IllegalPass(self.clone())),
            Ok(p) => {
                self.0 = p;
                Ok(San::null())
            }
        }
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Position {
    type Output = Option<Piece>;

    fn index(&self, s: Square) -> &Self::Output {
        use Color::*;
        use Role::*;
        match sm::Position::board(&self.0)
            .piece_at(s.into())
            .map(Into::into)
        {
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

impl TryFrom<Fen> for Position {
    type Error = IllegalPosition;

    fn try_from(fen: Fen) -> Result<Self, Self::Error> {
        Ok(Position(
            sm::Setup::from(fen).position(sm::CastlingMode::Standard)?,
        ))
    }
}

#[doc(hidden)]
impl From<Position> for sm::Setup {
    fn from(pos: Position) -> Self {
        sm::Position::into_setup(pos.0, sm::EnPassantMode::Always)
    }
}

#[doc(hidden)]
impl From<sm::Chess> for Position {
    fn from(chess: sm::Chess) -> Self {
        Position(chess)
    }
}

#[doc(hidden)]
impl From<Position> for sm::Chess {
    fn from(pos: Position) -> Self {
        pos.0
    }
}

#[doc(hidden)]
impl AsRef<sm::Chess> for Position {
    fn as_ref(&self) -> &sm::Chess {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::field::BitField;
    use proptest::sample::Selector;
    use std::collections::HashSet;
    use test_strategy::proptest;

    #[proptest]
    fn turn_returns_the_current_side_to_play(pos: Position) {
        assert_eq!(pos.turn(), sm::Setup::from(pos).turn.into());
    }

    #[proptest]
    fn halfmoves_returns_the_number_of_halfmoves_since_last_irreversible_move(pos: Position) {
        assert_eq!(pos.halfmoves(), sm::Setup::from(pos).halfmoves);
    }

    #[proptest]
    fn fullmoves_returns_the_current_move_number(pos: Position) {
        assert_eq!(pos.fullmoves(), sm::Setup::from(pos).fullmoves);
    }

    #[proptest]
    fn zobrist_returns_the_zobrist_hash(pos: Position) {
        assert_eq!(
            pos.zobrist().load::<u64>(),
            sm::zobrist::ZobristHash::zobrist_hash(&pos.0)
        );
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
    fn attacks_returns_squares_attacked_by_this_piece(pos: Position, s: Square) {
        for whither in pos.attacks(s) {
            assert!(pos
                .attackers(whither, pos[s].unwrap().color())
                .any(|whence| whence == s))
        }
    }

    #[proptest]
    fn attacks_returns_empty_iterator_if_square_is_not_occupied(
        #[by_ref] pos: Position,
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
            assert!(pos.attacks(whence).any(|whither| whither == s))
        }
    }

    #[proptest]
    fn checkers_returns_squares_of_pieces_giving_check(pos: Position) {
        assert_eq!(
            pos.checkers().collect::<Vec<_>>(),
            pos.by_piece(Piece(pos.turn(), Role::King))
                .flat_map(|s| pos.attackers(s, !pos.turn()))
                .collect::<Vec<_>>(),
        )
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
    fn moves_returns_all_legal_moves_from_this_position(pos: Position) {
        for (m, _, p) in pos.moves(MoveKind::ANY) {
            let mut pos = pos.clone();
            assert_eq!(pos[m.whence()].map(|p| p.color()), Some(pos.turn()));
            assert_eq!(pos.make(m).err(), None);
            assert_eq!(pos, p);
        }
    }

    #[proptest]
    fn captures_reduce_material(pos: Position) {
        for (_, _, p) in pos.moves(MoveKind::CAPTURE) {
            assert!(p.by_color(p.turn()).len() < pos.by_color(p.turn()).len());
        }
    }

    #[proptest]
    fn promotions_exchange_pawns(pos: Position) {
        for (_, _, p) in pos.moves(MoveKind::PROMOTION) {
            let pawn = Piece(pos.turn(), Role::Pawn);
            assert!(p.by_piece(pawn).len() < pos.by_piece(pawn).len());
            assert_eq!(p.by_color(pos.turn()).len(), pos.by_color(pos.turn()).len());
        }
    }

    #[proptest]
    fn castles_move_the_king_by_two_files(pos: Position) {
        for (m, _, _) in pos.moves(MoveKind::CASTLE) {
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
        #[by_ref]
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)]
        mut pos: Position,
        selector: Selector,
    ) {
        let (m, _, next) = selector.select(pos.moves(MoveKind::ANY));
        let vm = sm::uci::Uci::to_move(&m.into(), &pos.0)?;
        let san = sm::san::San::from_move(&pos.0, &vm).into();
        assert_eq!(pos.make(m), Ok(san));
        assert_eq!(pos, next);
    }

    #[proptest]
    fn illegal_move_fails_without_changing_position(
        #[by_ref] mut pos: Position,
        #[filter(#pos.clone().make(#m).is_err())] m: Move,
    ) {
        let before = pos.clone();
        assert_eq!(pos.make(m), Err(IllegalMove(m, before.clone())));
        assert_eq!(pos, before);
    }

    #[proptest]
    fn position_can_be_indexed_by_square(pos: Position, s: Square) {
        assert_eq!(
            pos[s],
            sm::Position::board(&pos.0)
                .piece_at(s.into())
                .map(Into::into)
        );
    }

    #[proptest]
    fn all_positions_can_be_represented_using_fen_notation(pos: Position) {
        assert_eq!(Position::try_from(Fen::from(pos.clone())), Ok(pos));
    }

    #[proptest]
    fn position_has_an_equivalent_shakmaty_representation(pos: Position) {
        assert_eq!(Position::from(sm::Chess::from(pos.clone())), pos);
    }
}
