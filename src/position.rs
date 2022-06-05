use crate::{Color, Fen, IllegalMove, Move, Piece, Role, San, Square};
use derive_more::{DebugCustom, Display, Error};
use shakmaty as sm;
use std::hash::{Hash, Hasher};
use std::{convert::TryFrom, num::NonZeroU32, ops::Index};

#[cfg(test)]
use proptest::{prelude::*, sample::Selector};

#[cfg(test)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PositionKind {
    Checkmate,
    Stalemate,
    InsufficientMaterial,
    Any,
}

#[cfg(test)]
impl Default for PositionKind {
    fn default() -> Self {
        PositionKind::Any
    }
}

/// The current position on the chess board.
///
/// This type guarantees that it only holds valid positions.
#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = PositionKind))]
#[debug(fmt = "Position(\"{}\")", self)]
#[display(fmt = "{}", "Fen::from(self.clone())")]
pub struct Position {
    #[cfg_attr(test, strategy(Self::strategy(&args).prop_map_into()))]
    chess: sm::Chess,
}

impl Position {
    #[cfg(test)]
    fn checkmate() -> impl Strategy<Value = Position> {
        prop_oneof![
            "4k3/8/8/8/8/2P1q3/1P6/3RKR2 w - - 0 1",
            "4R3/4kp2/5N2/4P3/8/8/8/4K3 b - - 0 1",
            "4k3/8/8/8/3nn3/3P4/4Kb2/3Q1B2 w - - 0 1",
            "3q1b2/4kB2/3p4/4N1B1/8/8/8/4K3 b - - 0 1",
            "4k1r1/8/8/8/8/5b2/5P1P/5RK1 w - - 0 1",
            "1nbB4/1pk5/2p5/8/8/8/8/3RK3 b - - 0 1",
            "4k3/8/2b5/8/8/8/4nP1P/5RK1 w - - 0 1",
            "3r1r2/4k2p/R3Q3/8/8/8/8/4K3 b - - 0 1",
        ]
        .prop_map(|s| s.parse().unwrap())
        .prop_map(|fen: Fen| fen.try_into().unwrap())
    }

    #[cfg(test)]
    fn stalemate() -> impl Strategy<Value = Position> {
        prop_oneof![
            "rn2k1nr/pp4pp/3p4/q1pP4/P1P2p1b/1b2pPRP/1P1NP1PQ/2B1KBNR w Kkq - 0 1",
            "5bnr/4p1pq/4Qpkr/7p/7P/4P3/PPPP1PP1/RNB1KBNR b KQ - 0 1",
            "NBk5/PpP1p3/1P2P3/8/8/3p1p2/3PpPpp/4Kbrq w - - 0 1",
            "NBk5/PpP1p3/1P2P3/8/8/3p1p2/3PpPpp/4Kbrq b - - 0 1",
            "8/8/8/2p2p1p/2P2P1k/4pP1P/4P1KP/5BNR w - - 0 1",
            "8/8/8/2p2p1p/2P2P1k/4pP1P/4P1KP/5BNR b - - 0 1",
            "8/8/8/8/pp6/kp6/1p6/1K6 w - - 0 1",
            "8/8/8/8/pp6/kp6/1p6/1K6 b - - 0 1",
        ]
        .prop_map(|s| s.parse().unwrap())
        .prop_map(|fen: Fen| fen.try_into().unwrap())
    }

    #[cfg(test)]
    fn insufficient_material() -> impl Strategy<Value = Position> {
        prop_oneof![
            "8/2K1k3/8/8/8/8/8/8 w - - 0 1",
            "8/8/8/1K6/8/1k6/8/8 b - - 0 1",
            "8/8/6k1/8/2K5/8/6N1/8 b - - 0 1",
            "8/2n5/2k5/8/4K3/8/8/8 w - - 0 1",
            "8/8/8/5k2/8/2B1K3/8/8 b - - 0 1",
            "8/6b1/1k6/5K2/8/8/8/8 w - - 0 1",
            "8/8/3kb3/8/8/3BK3/8/8 b - - 0 1",
            "8/8/3bk3/8/8/3KB3/8/8 w - - 0 1",
        ]
        .prop_map(|s| s.parse().unwrap())
        .prop_map(|fen: Fen| fen.try_into().unwrap())
    }

    #[cfg(test)]
    fn any() -> impl Strategy<Value = Position> {
        (0..32, any::<Selector>()).prop_map(|(depth, selector)| {
            let mut chess = sm::Chess::default();
            for _ in 0..depth {
                if let Some(m) = selector.try_select(sm::Position::legal_moves(&chess)) {
                    sm::Position::play_unchecked(&mut chess, &m);
                } else {
                    break;
                }
            }
            chess.into()
        })
    }

    #[cfg(test)]
    fn strategy(kind: &PositionKind) -> impl Strategy<Value = Position> {
        match kind {
            PositionKind::Checkmate => Self::checkmate().boxed(),
            PositionKind::Stalemate => Self::stalemate().boxed(),
            PositionKind::InsufficientMaterial => Self::insufficient_material().boxed(),
            PositionKind::Any => Self::any().boxed(),
        }
    }

    /// The side to move.
    pub fn turn(&self) -> Color {
        sm::Position::turn(&self.chess).into()
    }

    /// The number of halfmoves since the last capture or pawn advance.
    ///
    /// It resets to 0 whenever a piece is captured or a pawn is moved.
    pub fn halfmoves(&self) -> u32 {
        sm::Position::halfmoves(&self.chess)
    }

    /// The current move number since the start of the game.
    ///
    /// It starts at 1, and is incremented after every move by black.
    pub fn fullmoves(&self) -> NonZeroU32 {
        sm::Position::fullmoves(&self.chess)
    }

    /// Whether this position is a [checkmate].
    ///
    /// [checkmate]: https://en.wikipedia.org/wiki/Glossary_of_chess#checkmate
    pub fn is_checkmate(&self) -> bool {
        sm::Position::is_checkmate(&self.chess)
    }

    /// Whether this position is a [stalemate].
    ///
    /// [stalemate]: https://en.wikipedia.org/wiki/Glossary_of_chess#stalemate
    pub fn is_stalemate(&self) -> bool {
        sm::Position::is_stalemate(&self.chess)
    }

    /// Whether this position has [insufficient material].
    ///
    /// [insufficient material]: https://en.wikipedia.org/wiki/Glossary_of_chess#insufficient_material
    pub fn is_material_insufficient(&self) -> bool {
        sm::Position::is_insufficient_material(&self.chess)
    }

    /// The [`Square`]s occupied by [`Piece`]s of a kind.
    pub fn pieces(&self, p: Piece) -> impl ExactSizeIterator<Item = Square> {
        sm::Position::board(&self.chess)
            .by_piece(p.into())
            .into_iter()
            .map(Square::from)
    }

    /// Legal [`Move`]s that can be played in this position
    pub fn moves(&self) -> impl ExactSizeIterator<Item = Move> {
        sm::Position::legal_moves(&self.chess)
            .into_iter()
            .map(|m| sm::uci::Uci::from_standard(&m).into())
    }

    /// Play a [`Move`] if legal in this position.
    pub fn play(&mut self, m: Move) -> Result<San, IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.chess) {
            Ok(vm) if sm::Position::is_legal(&self.chess, &vm) => {
                Ok(sm::san::SanPlus::from_move_and_play_unchecked(&mut self.chess, &vm).into())
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
        state.write_u64(sm::zobrist::ZobristHash::zobrist_hash(&self.chess));
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Position {
    type Output = Option<Piece>;

    fn index(&self, s: Square) -> &Self::Output {
        use Color::*;
        use Role::*;
        match sm::Position::board(&self.chess)
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
            chess: sm::Setup::from(fen).position(sm::CastlingMode::Standard)?,
        })
    }
}

#[doc(hidden)]
impl From<Position> for sm::Setup {
    fn from(pos: Position) -> Self {
        sm::Position::into_setup(pos.chess, sm::EnPassantMode::Always)
    }
}

#[doc(hidden)]
impl From<sm::Chess> for Position {
    fn from(chess: sm::Chess) -> Self {
        Position { chess }
    }
}

#[doc(hidden)]
impl From<Position> for sm::Chess {
    fn from(pos: Position) -> Self {
        pos.chess
    }
}

#[doc(hidden)]
impl AsRef<sm::Chess> for Position {
    fn as_ref(&self) -> &sm::Chess {
        &self.chess
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::sample::select;
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
    fn is_checkmate_returns_whether_the_position_is_a_checkmate(
        #[any(PositionKind::Checkmate)] pos: Position,
    ) {
        assert!(pos.is_checkmate());
    }

    #[proptest]
    fn is_stalemate_returns_whether_the_position_is_a_stalemate(
        #[any(PositionKind::Stalemate)] pos: Position,
    ) {
        assert!(pos.is_stalemate());
    }

    #[proptest]
    fn is_material_insufficient_returns_whether_the_position_has_insufficient_material(
        #[any(PositionKind::InsufficientMaterial)] pos: Position,
    ) {
        assert!(pos.is_material_insufficient());
    }

    #[proptest]
    fn pieces_returns_squares_of_pieces_of_a_kind(pos: Position, p: Piece) {
        for s in pos.pieces(p) {
            assert_eq!(pos[s], Some(p));
        }
    }

    #[proptest]
    fn moves_returns_the_legal_moves_from_this_position(pos: Position) {
        let moves: Vec<Move> = sm::Position::legal_moves(&pos.chess)
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
        let vm = sm::uci::Uci::to_move(&m.into(), &pos.chess)?;
        let san = sm::san::SanPlus::from_move(pos.chess.clone(), &vm).into();
        let after = sm::Position::play(pos.chess.clone(), &vm)?.into();
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
            sm::Position::board(&pos.chess)
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
