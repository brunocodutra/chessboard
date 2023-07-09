use crate::{Color, Fen, Move, MoveContext, MoveKind, Outcome, Piece, Role, Square};
use derive_more::{DebugCustom, Display, Error};
use proptest::{prelude::*, sample::Selector};
use shakmaty as sm;
use std::{convert::TryFrom, num::NonZeroU32, ops::Index};
use test_strategy::Arbitrary;
use util::Bits;

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
#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[debug(fmt = "Position({self})")]
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
    }))]
    sm::Chess,
);

impl Position {
    /// The side to move.
    #[inline]
    pub fn turn(&self) -> Color {
        sm::Position::turn(&self.0).into()
    }

    /// The number of halfmoves since the last capture or pawn advance.
    ///
    /// It resets to 0 whenever a piece is captured or a pawn is moved.
    #[inline]
    pub fn halfmoves(&self) -> u32 {
        sm::Position::halfmoves(&self.0)
    }

    /// The current move number since the start of the game.
    ///
    /// It starts at 1, and is incremented after every move by black.
    #[inline]
    pub fn fullmoves(&self) -> NonZeroU32 {
        sm::Position::fullmoves(&self.0)
    }

    /// The en passant square.
    #[inline]
    pub fn en_passant_square(&self) -> Option<Square> {
        sm::Position::ep_square(&self.0, sm::EnPassantMode::Always).map(Square::from)
    }

    /// This position's [zobrist hash].
    ///
    /// [zobrist hash]: https://www.chessprogramming.org/Zobrist_Hashing
    #[inline]
    pub fn zobrist(&self) -> Zobrist {
        let z: sm::zobrist::Zobrist64 =
            sm::zobrist::ZobristHash::zobrist_hash(&self.0, sm::EnPassantMode::Always);
        Bits::new(z.0)
    }

    /// This position's [`Fen`] representation.
    #[inline]
    pub fn fen(&self) -> Fen {
        sm::Position::into_setup(self.0.clone(), sm::EnPassantMode::Always).into()
    }

    /// An iterator over all pieces on the board.
    #[inline]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (Piece, Square)> + ExactSizeIterator {
        sm::Position::board(&self.0)
            .clone()
            .into_iter()
            .map(|(s, p)| (p.into(), s.into()))
    }

    /// [`Square`]s occupied.
    #[inline]
    pub fn occupied(&self) -> impl DoubleEndedIterator<Item = Square> + ExactSizeIterator {
        sm::Position::board(&self.0)
            .occupied()
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`]s occupied by a [`Color`].
    #[inline]
    pub fn by_color(
        &self,
        c: Color,
    ) -> impl DoubleEndedIterator<Item = Square> + ExactSizeIterator {
        sm::Position::board(&self.0)
            .by_color(c.into())
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`]s occupied by a [`Role`].
    #[inline]
    pub fn by_role(&self, r: Role) -> impl DoubleEndedIterator<Item = Square> + ExactSizeIterator {
        sm::Position::board(&self.0)
            .by_role(r.into())
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`]s occupied by a [`Piece`].
    #[inline]
    pub fn by_piece(
        &self,
        p: Piece,
    ) -> impl DoubleEndedIterator<Item = Square> + ExactSizeIterator {
        sm::Position::board(&self.0)
            .by_piece(p.into())
            .into_iter()
            .map(Square::from)
    }

    /// [`Square`] occupied by a the king of the given color.
    #[inline]
    pub fn king(&self, side: Color) -> Square {
        self.by_piece(Piece(side, Role::King))
            .next()
            .expect("expected king on the board")
    }

    /// Whether this position is a [check].
    ///
    /// [check]: https://www.chessprogramming.org/Check
    #[inline]
    pub fn is_check(&self) -> bool {
        sm::Position::is_check(&self.0)
    }

    /// Whether this position is a [checkmate].
    ///
    /// [checkmate]: https://www.chessprogramming.org/Checkmate
    #[inline]
    pub fn is_checkmate(&self) -> bool {
        sm::Position::is_checkmate(&self.0)
    }

    /// Whether this position is a [stalemate].
    ///
    /// [stalemate]: https://www.chessprogramming.org/Stalemate
    #[inline]
    pub fn is_stalemate(&self) -> bool {
        sm::Position::is_stalemate(&self.0)
    }

    /// Whether this position has [insufficient material].
    ///
    /// [insufficient material]: https://www.chessprogramming.org/Material#InsufficientMaterial
    #[inline]
    pub fn is_material_insufficient(&self) -> bool {
        sm::Position::is_insufficient_material(&self.0)
    }

    /// The [`Outcome`] of the game in case this position is final.
    #[inline]
    pub fn outcome(&self) -> Option<Outcome> {
        if self.is_checkmate() {
            Some(Outcome::Checkmate(!self.turn()))
        } else if self.is_stalemate() {
            Some(Outcome::Stalemate)
        } else if self.halfmoves() >= 100 {
            Some(Outcome::DrawBy50MoveRule)
        } else if self.is_material_insufficient() {
            Some(Outcome::DrawByInsufficientMaterial)
        } else {
            None
        }
    }

    /// An iterator over the legal [`Move`]s that can be played in this position.
    #[inline]
    pub fn moves(
        &self,
        kind: MoveKind,
    ) -> impl DoubleEndedIterator<Item = (MoveContext, Self)> + ExactSizeIterator + '_ {
        let mut legals = sm::Position::legal_moves(&self.0);
        legals.retain(|vm| kind.intersects(vm.into()));
        legals.into_iter().map(move |vm| {
            let mut pos = self.clone();
            sm::Position::play_unchecked(&mut pos.0, &vm);
            (vm.into(), pos)
        })
    }

    /// Play a [`Move`] if legal in this position.
    #[inline]
    pub fn play(&mut self, m: Move) -> Result<MoveContext, IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.0) {
            Ok(vm) if sm::Position::is_legal(&self.0, &vm) => {
                sm::Position::play_unchecked(&mut self.0, &vm);
                Ok(vm.into())
            }

            _ => Err(IllegalMove(m)),
        }
    }

    /// Play a [null-move] if legal in this position.
    ///
    /// [null-move]: https://www.chessprogramming.org/Null_Move
    #[inline]
    pub fn pass(&mut self) -> Result<(), ImpossiblePass> {
        if self.is_check() {
            Err(ImpossiblePass)
        } else {
            let null = sm::Move::Put {
                role: sm::Role::King,
                to: self.king(self.turn()).into(),
            };

            sm::Position::play_unchecked(&mut self.0, &null);
            Ok(())
        }
    }

    /// Exchange a piece on [`Square`] by the attacker of least value.
    #[inline]
    pub fn exchange(&mut self, whither: Square) -> Result<MoveContext, ImpossibleExchange> {
        let to = whither.into();
        let board = sm::Position::board(&self.0);
        let capture = board.role_at(to).ok_or(ImpossibleExchange(whither))?;

        let (from, role) = board
            .attacks_to(to, sm::Position::turn(&self.0), board.occupied())
            .into_iter()
            .filter_map(|s| Some((s, board.role_at(s)?)))
            .min_by_key(|(_, r)| *r)
            .ok_or(ImpossibleExchange(whither))?;

        let promotion = match (role, to.rank()) {
            (sm::Role::Pawn, sm::Rank::First | sm::Rank::Eighth) => Some(sm::Role::Queen),
            _ => None,
        };

        let vm = sm::Move::Normal {
            role,
            from,
            capture: Some(capture),
            to,
            promotion,
        };

        if sm::Position::is_legal(&self.0, &vm) {
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

    #[inline]
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
    #[inline]
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

    #[inline]
    fn try_from(fen: Fen) -> Result<Self, Self::Error> {
        Ok(Position(
            sm::Setup::from(fen).position(sm::CastlingMode::Standard)?,
        ))
    }
}

#[doc(hidden)]
impl From<sm::Chess> for Position {
    #[inline]
    fn from(chess: sm::Chess) -> Self {
        Position(chess)
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
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Always);
        assert_eq!(pos.turn(), setup.turn.into());
    }

    #[proptest]
    fn halfmoves_returns_the_number_of_halfmoves_since_last_irreversible_move(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Always);
        assert_eq!(pos.halfmoves(), setup.halfmoves);
    }

    #[proptest]
    fn fullmoves_returns_the_current_move_number(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Always);
        assert_eq!(pos.fullmoves(), setup.fullmoves);
    }

    #[proptest]
    fn en_passant_square_returns_the_en_passant_square(pos: Position) {
        let setup = sm::Position::into_setup(pos.0.clone(), sm::EnPassantMode::Always);
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
    fn moves_returns_all_legal_moves_from_this_position(pos: Position) {
        for (m, p) in pos.moves(MoveKind::ANY) {
            let mut pos = pos.clone();
            assert_eq!(pos[m.whence()].map(|p| p.color()), Some(pos.turn()));
            assert_eq!(pos.play(*m).err(), None);
            assert_eq!(pos, p);
        }
    }

    #[proptest]
    fn captures_reduce_material(pos: Position) {
        for (_, p) in pos.moves(MoveKind::CAPTURE) {
            assert!(p.by_color(p.turn()).len() < pos.by_color(p.turn()).len());
        }
    }

    #[proptest]
    fn promotions_exchange_pawns(pos: Position) {
        for (_, p) in pos.moves(MoveKind::PROMOTION) {
            let pawn = Piece(pos.turn(), Role::Pawn);
            assert!(p.by_piece(pawn).len() < pos.by_piece(pawn).len());
            assert_eq!(p.by_color(pos.turn()).len(), pos.by_color(pos.turn()).len());
        }
    }

    #[proptest]
    fn castles_move_the_king_by_two_files(pos: Position) {
        for (m, _) in pos.moves(MoveKind::CASTLE) {
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
        #[filter(#pos.clone().moves(MoveKind::ANY).len() > 0)] mut pos: Position,
        selector: Selector,
    ) {
        let (m, next) = selector.select(pos.moves(MoveKind::ANY));
        assert_eq!(pos.play(*m).map(MoveContext::from), Ok(m));
        assert_eq!(pos, next);
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
    fn pass_updates_position(#[filter(!#pos.is_check())] mut pos: Position) {
        let before = pos.clone();
        assert_eq!(pos.pass(), Ok(()));
        assert_ne!(pos, before);
    }

    #[proptest]
    fn impossible_pass_fails_without_changing_position(
        #[filter(#pos.clone().pass().is_err())] mut pos: Position,
    ) {
        let before = pos.clone();
        assert_eq!(pos.pass(), Err(ImpossiblePass));
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
}
