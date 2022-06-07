use crate::{Color, Fen, IllegalMove, Move, Piece, Role, San, Square};
use derive_more::{DebugCustom, Display, Error};
use shakmaty as sm;
use std::hash::{Hash, Hasher};
use std::{convert::TryFrom, num::NonZeroU32, ops::Index};

#[cfg(test)]
use proptest::{prelude::*, sample::Selector};

/// The current position on the chess board.
///
/// This type guarantees that it only holds valid positions.
#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "Position(\"{}\")", self)]
#[display(fmt = "{}", "Fen::from(self.clone())")]
pub struct Position {
    #[cfg_attr(test, strategy((0..32, any::<Selector>()).prop_map(|(depth, selector)| {
        let mut chess = sm::Chess::default();
        for _ in 0..depth {
            if let Some(m) = selector.try_select(sm::Position::legal_moves(&chess)) {
                sm::Position::play_unchecked(&mut chess, &m);
            } else {
                break;
            }
        }
        chess
    })))]
    pub(crate) board: sm::Chess,
}

impl Position {
    /// The side to move.
    pub fn turn(&self) -> Color {
        sm::Position::turn(&self.board).into()
    }

    /// The number of halfmoves since the last capture or pawn advance.
    ///
    /// It resets to 0 whenever a piece is captured or a pawn is moved.
    pub fn halfmoves(&self) -> u32 {
        sm::Position::halfmoves(&self.board)
    }

    /// The current move number since the start of the game.
    ///
    /// It starts at 1, and is incremented after every move by black.
    pub fn fullmoves(&self) -> NonZeroU32 {
        sm::Position::fullmoves(&self.board)
    }

    /// The [`Square`]s occupied by [`Piece`]s of a kind.
    pub fn pieces(&self, p: Piece) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.board)
            .by_piece(p.into())
            .into_iter()
            .map(Square::from)
    }

    /// Into where the piece in this [`Square`] can attack.
    pub fn attacks(&self, s: Square) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.board)
            .attacks_from(s.into())
            .into_iter()
            .map(Square::from)
    }

    /// From where pieces of this [`Color`] can attack into this [`Square`].
    pub fn attackers(&self, s: Square, c: Color) -> impl ExactSizeIterator<Item = Square> {
        let board = sm::Position::board(&self.board);
        board
            .attacks_to(s.into(), c.into(), board.occupied())
            .into_iter()
            .map(Square::from)
    }

    /// The [`Square`]s occupied by [`Piece`]s giving check.
    pub fn checkers(&self) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::checkers(&self.board)
            .into_iter()
            .map(Square::from)
    }

    /// Legal [`Move`]s that can be played in this position
    pub fn moves(&self) -> impl ExactSizeIterator<Item = Move> {
        sm::Position::legal_moves(&self.board)
            .into_iter()
            .map(|m| sm::uci::Uci::from_standard(&m).into())
    }

    /// Play a [`Move`] if legal in this position.
    pub fn play(&mut self, m: Move) -> Result<San, IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.board) {
            Ok(vm) if sm::Position::is_legal(&self.board, &vm) => {
                Ok(sm::san::SanPlus::from_move_and_play_unchecked(&mut self.board, &vm).into())
            }

            _ => Err(IllegalMove(m, self.clone())),
        }
    }
}

/// Computes the [Zobrist] hash.
///
/// [Zobrist]: https://en.wikipedia.org/wiki/Zobrist_hashing
#[allow(clippy::derive_hash_xor_eq)]
impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(sm::zobrist::ZobristHash::zobrist_hash(&self.board));
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Position {
    type Output = Option<Piece>;

    fn index(&self, s: Square) -> &Self::Output {
        use Color::*;
        use Role::*;
        match sm::Position::board(&self.board)
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
        Ok(Position {
            board: sm::Setup::from(fen).position(sm::CastlingMode::Standard)?,
        })
    }
}

#[doc(hidden)]
impl From<Position> for sm::Setup {
    fn from(pos: Position) -> Self {
        sm::Position::into_setup(pos.board, sm::EnPassantMode::Always)
    }
}

#[doc(hidden)]
impl From<sm::Chess> for Position {
    fn from(chess: sm::Chess) -> Self {
        Position { board: chess }
    }
}

#[doc(hidden)]
impl From<Position> for sm::Chess {
    fn from(pos: Position) -> Self {
        pos.board
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::select;
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
    fn pieces_returns_squares_of_pieces_of_a_kind(pos: Position, p: Piece) {
        for s in pos.pieces(p) {
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
            pos.checkers().collect::<HashSet<_>>(),
            pos.pieces(Piece(pos.turn(), Role::King))
                .flat_map(|s| pos.attackers(s, !pos.turn()))
                .collect::<HashSet<_>>(),
        )
    }

    #[proptest]
    fn moves_returns_the_legal_moves_from_this_position(pos: Position) {
        let moves: Vec<Move> = sm::Position::legal_moves(&pos.board)
            .iter()
            .map(sm::uci::Uci::from_standard)
            .map(Into::into)
            .collect();

        assert_eq!(pos.moves().collect::<Vec<_>>(), moves);
    }

    #[proptest]
    fn playing_legal_move_updates_position(
        #[by_ref]
        #[filter(#pos.moves().len() > 0)]
        mut pos: Position,
        #[strategy(select(#pos.moves().collect::<Vec<_>>()))] m: Move,
    ) {
        let vm = sm::uci::Uci::to_move(&m.into(), &pos.board)?;
        let san = sm::san::SanPlus::from_move(pos.board.clone(), &vm).into();
        let after = sm::Position::play(pos.board.clone(), &vm)?.into();
        assert_eq!(pos.play(m), Ok(san));
        assert_eq!(pos, after);
    }

    #[proptest]
    fn playing_illegal_move_fails(
        #[by_ref] mut pos: Position,
        #[filter(!#pos.moves().any(|n| n == #m))] m: Move,
    ) {
        let before = pos.clone();
        assert_eq!(pos.play(m), Err(IllegalMove(m, before.clone())));
        assert_eq!(pos, before);
    }

    #[proptest]
    fn position_can_be_indexed_by_square(pos: Position, s: Square) {
        assert_eq!(
            pos[s],
            sm::Position::board(&pos.board)
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
