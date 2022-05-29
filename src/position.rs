use crate::{Color, Fen, IllegalMove, Move, Placement, Square};
use derive_more::{DebugCustom, Display, Error};
use shakmaty as sm;
use std::{convert::TryFrom, num::NonZeroU32};

#[cfg(test)]
use proptest::{prelude::*, sample::Selector};

#[cfg(test)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PositionKind {
    Checkmate,
    Stalemate,
    Draw,
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
#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq, Hash)]
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
    fn draw() -> impl Strategy<Value = Position> {
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
        prop_oneof![
            Self::checkmate(),
            Self::stalemate(),
            Self::draw(),
            Just(sm::Chess::default())
                .prop_recursive(32, 32, 1, |inner| {
                    (inner, any::<Selector>()).prop_map(|(chess, selector)| {
                        if let Some(m) = selector.try_select(sm::Position::legal_moves(&chess)) {
                            sm::Position::play(chess, &m).unwrap()
                        } else {
                            chess
                        }
                    })
                })
                .no_shrink()
                .prop_map_into()
        ]
    }

    #[cfg(test)]
    fn strategy(kind: &PositionKind) -> impl Strategy<Value = Position> {
        match kind {
            PositionKind::Checkmate => Self::checkmate().boxed(),
            PositionKind::Stalemate => Self::stalemate().boxed(),
            PositionKind::Draw => Self::draw().boxed(),
            PositionKind::Any => Self::any().boxed(),
        }
    }

    /// The current arrangement of [`Piece`](crate::Piece)s on the board.
    pub fn placement(&self) -> Placement {
        sm::Position::board(&self.chess).clone().into()
    }

    /// The side to move.
    pub fn turn(&self) -> Color {
        sm::Position::turn(&self.chess).into()
    }

    /// The en passant target square, if any.
    ///
    /// The existence of an en passant square does not imply that the pushed pawn can be captured,
    /// but simply that a pawn has been pushed two squares forward.
    pub fn en_passant_square(&self) -> Option<Square> {
        sm::Position::maybe_ep_square(&self.chess).map(Into::into)
    }

    /// The number of halfmoves since the last capture or pawn advance.
    pub fn halfmove_clock(&self) -> u32 {
        sm::Position::halfmoves(&self.chess)
    }

    /// The current move number.
    ///
    /// It starts at 1, and is incremented after every Black's move.
    pub fn fullmoves(&self) -> NonZeroU32 {
        sm::Position::fullmoves(&self.chess)
    }

    /// Whether this position is a [checkmate].
    ///
    /// [checkmate]: https://en.wikipedia.org/wiki/Checkmate
    pub fn is_checkmate(&self) -> bool {
        sm::Position::is_checkmate(&self.chess)
    }

    /// Whether this position is a [stalemate].
    ///
    /// [stalemate]: https://en.wikipedia.org/wiki/Stalemate
    pub fn is_stalemate(&self) -> bool {
        sm::Position::is_stalemate(&self.chess)
    }

    /// Whether this position is a [draw] by insufficient material.
    ///
    /// [draw]: https://en.wikipedia.org/wiki/Draw_(chess)
    pub fn is_draw(&self) -> bool {
        sm::Position::is_insufficient_material(&self.chess)
    }

    /// Legal [`Move`]s that can be played in this position
    pub fn moves(&self) -> impl ExactSizeIterator<Item = Move> + Clone {
        sm::Position::legal_moves(&self.chess)
            .into_iter()
            .map(|m| sm::uci::Uci::from_standard(&m).into())
    }

    /// Play a [`Move`] if legal in this position.
    pub fn play(&mut self, m: Move) -> Result<(), IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.chess) {
            Ok(vm) if sm::Position::is_legal(&self.chess, &vm) => {
                sm::Position::play_unchecked(&mut self.chess, &vm);
                Ok(())
            }

            _ => Err(IllegalMove(m, self.clone())),
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
    fn placement_returns_the_piece_arrangement(pos: Position) {
        assert_eq!(pos.placement(), sm::Setup::from(pos).board.into());
    }

    #[proptest]
    fn turn_returns_the_current_side_to_play(pos: Position) {
        assert_eq!(pos.turn(), sm::Setup::from(pos).turn.into());
    }

    #[proptest]
    fn en_passant_square_returns_the_current_pushed_pawn_skipped_square(pos: Position) {
        assert_eq!(
            pos.en_passant_square(),
            sm::Setup::from(pos).ep_square.map(Into::into)
        );
    }

    #[proptest]
    fn halfmove_clock_returns_the_number_of_halfmoves_since_last_irreversible_move(pos: Position) {
        assert_eq!(pos.halfmove_clock(), sm::Setup::from(pos).halfmoves);
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
    fn is_draw_returns_whether_the_position_has_insufficient_material(
        #[any(PositionKind::Draw)] pos: Position,
    ) {
        assert!(pos.is_draw());
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
        let after = sm::Position::play(pos.chess.clone(), &vm)?.into();
        assert_eq!(pos.play(m), Ok(()));
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
    fn all_positions_can_be_represented_using_fen_notation(pos: Position) {
        assert_eq!(Position::try_from(Fen::from(pos.clone())), Ok(pos));
    }

    #[proptest]
    fn position_has_an_equivalent_shakmaty_representation(pos: Position) {
        assert_eq!(Position::from(sm::Chess::from(pos.clone())), pos);
    }
}
