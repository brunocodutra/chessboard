use crate::{Color, IllegalMove, Move, Placement, Square};
use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::{convert::TryFrom, num::NonZeroU32, str::FromStr};

#[cfg(test)]
use proptest::{prelude::*, sample::select};

#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "Fen(\"{}\")", self)]
#[display(fmt = "{}", "setup")]
struct Fen {
    #[cfg_attr(test, strategy(
        any::<Placement>().prop_filter_map("invalid fen", |p| {
            let fen = sm::fen::Fen(sm::Setup { board: p.into(), ..Default::default() });
            fen.to_string().parse().ok()
        })
    ))]
    setup: sm::fen::Fen,
}

impl FromStr for Fen {
    type Err = InvalidFen;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Fen { setup: s.parse()? })
    }
}

#[doc(hidden)]
impl TryFrom<Fen> for Position {
    type Error = IllegalPosition;

    fn try_from(fen: Fen) -> Result<Self, Self::Error> {
        Ok(Position {
            chess: fen.setup.into_position(sm::CastlingMode::Standard)?,
        })
    }
}

#[doc(hidden)]
impl From<Position> for Fen {
    fn from(pos: Position) -> Self {
        Fen {
            setup: sm::Position::into_setup(pos.chess, sm::EnPassantMode::Always).into(),
        }
    }
}

#[cfg(test)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum PositionKind {
    Opening,
    Ending,
    Checkmate,
    Stalemate,
    Draw,
    Any,
}

#[cfg(test)]
impl PositionKind {
    #[inline]
    fn opening() -> Vec<&'static str> {
        vec![
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "rnbqkb1r/ppp1pppp/3p4/3nP3/3P4/5N2/PPP2PPP/RNBQKB1R b KQkq - 1 4",
            "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            "rnbqkb1r/ppp1pp1p/3p1np1/8/3PPP2/2N5/PPP3PP/R1BQKBNR b KQkq - 0 4",
            "r1bqkb1r/pppp1ppp/2n2n2/1B2p3/4P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            "rnbqkb1r/ppp1pppp/5n2/3P4/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3",
            "rnbqkb1r/pp2pp1p/3p1np1/8/3NP3/2N5/PPP2PPP/R1BQKB1R w KQkq - 0 6",
            "rnbqkb1r/ppp1pp1p/5np1/3p4/2PP1B2/2N5/PP2PPPP/R2QKBNR b KQkq - 1 4",
        ]
    }

    #[inline]
    fn ending() -> Vec<&'static str> {
        vec![
            "8/8/8/3nk3/8/3Q4/8/7K w - - 0 1",
            "8/8/3B4/6K1/8/8/2k5/q7 b - - 0 1",
            "8/3k4/3P4/3K4/8/8/8/8 w - - 0 1",
            "3K4/3P1k2/8/8/8/8/7r/4R3 b - - 0 1",
            "3k4/4r3/3K4/3B4/8/8/8/5R2 w - - 0 1",
            "4k3/7R/8/3KPr2/8/8/8/8 b - - 0 1",
            "8/8/4r3/3k4/8/8/3K1Q2/8 w - - 0 1",
            "8/7k/6pP/6P1/8/p1p5/8/1K6 b - - 0 1",
        ]
    }

    #[inline]
    fn checkmate() -> Vec<&'static str> {
        vec![
            "4k3/8/8/8/8/2P1q3/1P6/3RKR2 w - - 0 1",
            "4R3/4kp2/5N2/4P3/8/8/8/4K3 b - - 0 1",
            "4k3/8/8/8/3nn3/3P4/4Kb2/3Q1B2 w - - 0 1",
            "3q1b2/4kB2/3p4/4N1B1/8/8/8/4K3 b - - 0 1",
            "4k1r1/8/8/8/8/5b2/5P1P/5RK1 w - - 0 1",
            "1nbB4/1pk5/2p5/8/8/8/8/3RK3 b - - 0 1",
            "4k3/8/2b5/8/8/8/4nP1P/5RK1 w - - 0 1",
            "3r1r2/4k2p/R3Q3/8/8/8/8/4K3 b - - 0 1",
        ]
    }

    #[inline]
    fn stalemate() -> Vec<&'static str> {
        vec![
            "rn2k1nr/pp4pp/3p4/q1pP4/P1P2p1b/1b2pPRP/1P1NP1PQ/2B1KBNR w Kkq - 0 1",
            "5bnr/4p1pq/4Qpkr/7p/7P/4P3/PPPP1PP1/RNB1KBNR b KQ - 0 1",
            "NBk5/PpP1p3/1P2P3/8/8/3p1p2/3PpPpp/4Kbrq w - - 0 1",
            "NBk5/PpP1p3/1P2P3/8/8/3p1p2/3PpPpp/4Kbrq b - - 0 1",
            "8/8/8/2p2p1p/2P2P1k/4pP1P/4P1KP/5BNR w - - 0 1",
            "8/8/8/2p2p1p/2P2P1k/4pP1P/4P1KP/5BNR b - - 0 1",
            "8/8/8/8/pp6/kp6/1p6/1K6 w - - 0 1",
            "8/8/8/8/pp6/kp6/1p6/1K6 b - - 0 1",
        ]
    }

    #[inline]
    fn draw() -> Vec<&'static str> {
        vec![
            "8/2K1k3/8/8/8/8/8/8 w - - 0 1",
            "8/8/8/1K6/8/1k6/8/8 b - - 0 1",
            "8/8/6k1/8/2K5/8/6N1/8 b - - 0 1",
            "8/2n5/2k5/8/4K3/8/8/8 w - - 0 1",
            "8/8/8/5k2/8/2B1K3/8/8 b - - 0 1",
            "8/6b1/1k6/5K2/8/8/8/8 w - - 0 1",
            "8/8/3kb3/8/8/3BK3/8/8 b - - 0 1",
            "8/8/3bk3/8/8/3KB3/8/8 w - - 0 1",
        ]
    }

    #[inline]
    fn any() -> Vec<&'static str> {
        [
            Self::opening(),
            Self::ending(),
            Self::checkmate(),
            Self::stalemate(),
            Self::draw(),
        ]
        .concat()
    }
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
    fn strategy(kind: &PositionKind) -> impl Strategy<Value = Self> {
        let positions = match kind {
            PositionKind::Opening => select(PositionKind::opening()),
            PositionKind::Ending => select(PositionKind::ending()),
            PositionKind::Checkmate => select(PositionKind::checkmate()),
            PositionKind::Stalemate => select(PositionKind::stalemate()),
            PositionKind::Draw => select(PositionKind::draw()),
            PositionKind::Any => select(PositionKind::any()),
        };

        positions
            .prop_map(|s| s.parse().unwrap())
            .prop_map(|fen: Fen| Position::try_from(fen).unwrap())
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

/// The reason why a FEN string is invalid.
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

/// The reason why the position represented by a FEN string is illegal.
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

/// The reason why parsing [`Position`] from a FEN string failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
pub enum ParsePositionError {
    #[display(fmt = "failed to parse FEN")]
    InvalidFen(InvalidFen),

    #[display(fmt = "FEN represents an illegal position")]
    IllegalPosition(IllegalPosition),
}

/// Parses a [`Position`] from a FEN string.
impl FromStr for Position {
    type Err = ParsePositionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let fen: Fen = s.parse()?;
        Ok(Self::try_from(fen)?)
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
impl From<Position> for sm::Setup {
    fn from(pos: Position) -> Self {
        Fen::from(pos).setup.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn parsing_printed_position_is_an_identity(pos: Position) {
        assert_eq!(pos.to_string().parse(), Ok(pos));
    }

    #[proptest]
    fn parsing_invalid_fen_fails(
        #[by_ref] fen: Fen,
        #[strategy(..=#fen.to_string().len())] n: usize,
        #[strategy(".+")] r: String,
    ) {
        let s = [&fen.to_string()[..n], &r].concat();
        let result = s.parse::<Fen>();
        prop_assume!(result.is_err());
        assert_eq!(s.parse::<Position>().err(), result.err().map(Into::into));
    }

    #[proptest]
    fn parsing_illegal_fen_fails(fen: Fen) {
        let s = fen.to_string();
        let result = Position::try_from(fen);
        prop_assume!(result.is_err());
        assert_eq!(s.parse::<Position>().err(), result.err().map(Into::into));
    }

    #[proptest]
    fn position_has_an_equivalent_shakmaty_representation(pos: Position) {
        assert_eq!(Position::from(sm::Chess::from(pos.clone())), pos);
    }
}
