use crate::{Color, IllegalMove, Move, Placement, Square};
use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::hash::{Hash, Hasher};
use std::{num::NonZeroU32, str::FromStr};
use tracing::instrument;

#[cfg(test)]
use proptest::prelude::*;

/// The current position on the chess board.
///
/// This type guarantees that it only holds valid positions.
#[derive(DebugCustom, Display, Default, Clone)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[debug(fmt = "Position(\"{}\")", self)]
#[display(fmt = "{}", "sm::fen::FenOpts::new().promoted(true).fen(setup)")]
pub struct Position {
    #[cfg_attr(test, proptest(strategy = "tests::any_setup()"))]
    setup: sm::Chess,
}

#[cfg(test)]
#[derive(Debug, derive_more::Deref, derive_more::Into, proptest_derive::Arbitrary)]
pub struct Checkmate(#[proptest(strategy = "tests::checkmate_setup().prop_map_into()")] Position);

#[cfg(test)]
#[derive(Debug, derive_more::Deref, derive_more::Into, proptest_derive::Arbitrary)]
pub struct Stalemate(#[proptest(strategy = "tests::stalemate_setup().prop_map_into()")] Position);

#[cfg(test)]
#[derive(Debug, derive_more::Deref, derive_more::Into, proptest_derive::Arbitrary)]
pub struct Draw(#[proptest(strategy = "tests::draw_setup().prop_map_into()")] Position);

impl Position {
    /// The current arrangement of [`Piece`](crate::Piece)s on the board.
    pub fn placement(&self) -> Placement {
        sm::Setup::board(&self.setup).clone().into()
    }

    /// The side to move.
    pub fn turn(&self) -> Color {
        sm::Setup::turn(&self.setup).into()
    }

    /// The en passant target square, if any.
    pub fn en_passant_square(&self) -> Option<Square> {
        sm::Setup::ep_square(&self.setup).map(Into::into)
    }

    /// The number of halfmoves since the last capture or pawn advance.
    pub fn halfmove_clock(&self) -> u32 {
        sm::Setup::halfmoves(&self.setup)
    }

    /// The current move number.
    ///
    /// It starts at 1, and is incremented after every Black's move.
    pub fn fullmoves(&self) -> NonZeroU32 {
        sm::Setup::fullmoves(&self.setup)
    }

    /// Whether this position is a [checkmate].
    ///
    /// [checkmate]: https://en.wikipedia.org/wiki/Checkmate
    pub fn is_checkmate(&self) -> bool {
        sm::Position::is_checkmate(&self.setup)
    }

    /// Whether this position is a [stalemate].
    ///
    /// [stalemate]: https://en.wikipedia.org/wiki/Stalemate
    pub fn is_stalemate(&self) -> bool {
        sm::Position::is_stalemate(&self.setup)
    }

    /// Whether this position is a [draw] by insufficient material.
    ///
    /// [draw]: https://en.wikipedia.org/wiki/Draw_(chess)
    pub fn is_draw(&self) -> bool {
        sm::Position::is_insufficient_material(&self.setup)
    }

    /// Legal [`Move`]s that can be played in this position
    pub fn moves(&self) -> impl ExactSizeIterator<Item = Move> + Clone {
        sm::Position::legal_moves(&self.setup)
            .into_iter()
            .map(|m| sm::uci::Uci::from_standard(&m).into())
    }

    /// Play a [`Move`] if legal in this position.
    #[instrument(level = "trace", err)]
    pub fn play(&mut self, m: Move) -> Result<(), IllegalMove> {
        match sm::uci::Uci::to_move(&m.into(), &self.setup) {
            Ok(vm) if sm::Position::is_legal(&self.setup, &vm) => {
                sm::Position::play_unchecked(&mut self.setup, &vm);
                Ok(())
            }

            _ => Err(IllegalMove(m, self.clone())),
        }
    }
}

impl Eq for Position {}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        sm::fen::Fen::from_setup(&self.setup) == sm::fen::Fen::from_setup(&other.setup)
    }
}

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        sm::fen::Fen::from_setup(&self.setup).hash(state);
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
    #[display(fmt = "there are pawns on the backrank")]
    PawnsOnBackrank,
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
            IllegalPosition::PawnsOnBackrank
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
    #[display(fmt = "unable to parse FEN")]
    InvalidFen(InvalidFen),

    #[display(fmt = "FEN represents an illegal position")]
    IllegalPosition(IllegalPosition),
}

/// Parses a [`Position`] from a FEN string.
impl FromStr for Position {
    type Err = ParsePositionError;

    #[instrument(level = "trace", err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParsePositionError::*;

        let fen: sm::fen::Fen = s
            .parse()
            .map_err(|e: sm::fen::ParseFenError| InvalidFen(e.into()))?;

        let setup: sm::Chess = fen
            .position(sm::CastlingMode::Standard)
            .map_err(|e| IllegalPosition(e.into()))?;

        Ok(Position { setup })
    }
}

#[doc(hidden)]
impl From<sm::Chess> for Position {
    fn from(setup: sm::Chess) -> Self {
        Position { setup }
    }
}

#[doc(hidden)]
impl From<Position> for sm::Chess {
    fn from(pos: Position) -> Self {
        pos.setup
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Square;
    use proptest::collection::hash_set;
    use proptest::sample::{select, Selector};
    use std::num::NonZeroU32;

    pub(super) fn any_setup() -> impl Strategy<Value = sm::Chess> {
        prop_oneof![
            checkmate_setup(),
            stalemate_setup(),
            draw_setup(),
            Just(sm::Chess::default())
                .prop_recursive(32, 1048576, 1, |inner| {
                    (inner, any::<Selector>()).prop_map(|(setup, selector)| {
                        if let Some(m) = selector.try_select(sm::Position::legal_moves(&setup)) {
                            sm::Position::play(setup, &m).unwrap()
                        } else {
                            setup
                        }
                    })
                })
                .no_shrink()
        ]
    }

    pub(super) fn checkmate_setup() -> impl Strategy<Value = sm::Chess> {
        let positions = vec![
            "8/4N1pk/8/7R/8/8/8/4K3 b - - 0 1",
            "6kR/6P1/5K2/8/8/8/8/8 b - - 0 1",
            "7k/7R/5N2/8/8/8/8/4K3 b - - 0 1",
            "3R2k1/5ppp/8/8/8/8/8/4K3 b - - 0 1",
            "7k/8/5BKN/8/8/8/8/8 b - - 0 1",
            "5rk1/7B/8/6N1/8/8/1B6/4K3 b - - 0 1",
            "7k/5B2/8/6N1/8/8/1B6/4K3 b - - 0 1",
            "5rk1/6RR/8/8/8/8/8/4K3 b - - 0 1",
            "2kr4/3p4/B7/8/5B2/8/8/4K3 b - - 0 1",
            "R2k4/8/3K4/8/8/8/8/8 b - - 0 1",
            "7k/5N1p/8/8/8/8/8/4K1R1 b - - 0 1",
            "8/8/8/8/6p1/5qk1/7Q/6K1 b - - 0 1",
            "5k2/5Q2/6B1/8/8/8/8/4K3 b - - 0 1",
            "5rk1/6pQ/6P1/8/8/8/8/4K3 b - - 0 1",
            "8/8/8/7R/pkp5/Pn6/1P6/4K3 b - - 0 1",
            "7k/7p/8/3B4/8/2B5/8/4K3 b - - 0 1",
            "3rkr2/1p6/2p1Q3/8/8/8/8/4K3 b - - 0 1",
            "7k/6p1/8/7Q/2B5/8/8/4K3 b - - 0 1",
            "4R3/4kp2/5N2/4P3/8/8/8/4K3 b - - 0 1",
            "8/8/R7/k7/2Q5/8/8/4K3 b - - 0 1",
            "7k/8/4BB1K/8/8/8/8/8 b - - 0 1",
            "7k/8/5NNK/8/8/8/8/8 b - - 0 1",
            "R6k/1R6/8/8/8/8/8/4K3 b - - 0 1",
            "Q~6k/1R6/8/8/8/8/8/4K3 b - - 0 1",
            "3q1b2/4kB2/3p4/3NN3/8/8/8/4K3 b - - 0 1",
            "3q1b2/4kB2/3p4/4N1B1/8/8/8/4K3 b - - 0 1",
            "5k2/4pQ2/4Pp2/8/8/8/8/4K3 b - - 0 1",
            "6Q1/5Bpk/7p/8/8/8/8/4K3 b - - 0 1",
            "6kR/5p2/8/8/8/8/1B6/4K3 b - - 0 1",
            "7k/7p/5B2/8/8/8/8/4K1R1 b - - 0 1",
            "3Rk3/5p2/8/6B1/8/8/8/4K3 b - - 0 1",
            "5rk1/5p1p/5B2/8/8/8/8/4K1R1 b - - 0 1",
            "3k4/3Q4/3K4/8/8/8/8/8 b - - 0 1",
            "1nbB4/1pk5/2p5/8/8/8/8/3RK3 b - - 0 1",
            "6rk/5Npp/8/8/8/8/8/4K3 b - - 0 1",
            "5rk1/4Np1p/8/8/8/2B5/8/4K3 b - - 0 1",
            "3r1r2/4k2p/R3Q3/8/8/8/8/4K3 b - - 0 1",
        ];

        select(positions)
            .prop_filter_map("invalid fen", |s| s.parse().ok())
            .prop_filter_map("illegal position", |fen: sm::fen::Fen| {
                fen.position(sm::CastlingMode::Standard).ok()
            })
    }

    pub(super) fn stalemate_setup() -> impl Strategy<Value = sm::Chess> {
        let positions = vec![
            "rn2k1nr/pp4pp/3p4/q1pP4/P1P2p1b/1b2pPRP/1P1NP1PQ/2B1KBNR w Kkq - 0 1",
            "5bnr/4p1pq/4Qpkr/7p/7P/4P3/PPPP1PP1/RNB1KBNR b KQ - 0 1",
            "NBk5/PpP1p3/1P2P3/8/8/3p1p2/3PpPpp/4Kbrq w - - 0 1",
            "NBk5/PpP1p3/1P2P3/8/8/3p1p2/3PpPpp/4Kbrq b - - 0 1",
            "8/8/8/2p2p1p/2P2P1k/4pP1P/4P1KP/5BNR w - - 0 1",
            "8/8/8/2p2p1p/2P2P1k/4pP1P/4P1KP/5BNR b - - 0 1",
            "8/8/8/8/pp6/kp6/1p6/1K6 w - - 0 1",
            "8/8/8/8/pp6/kp6/1p6/1K6 b - - 0 1",
            "5k2/5P2/5K2/8/8/8/8/8 b - - 0 1",
            "kb5R/8/1K6/8/8/8/8/8 b - - 0 1",
            "8/8/8/8/8/2K5/1R6/k7 b - - 0 1",
            "8/8/8/6K1/8/1Q6/p7/k7 b - - 0 1",
            "k7/P7/K7/8/5B2/8/8/8 b - - 0 1",
        ];

        select(positions)
            .prop_filter_map("invalid fen", |s| s.parse().ok())
            .prop_filter_map("illegal position", |fen: sm::fen::Fen| {
                fen.position(sm::CastlingMode::Standard).ok()
            })
    }

    pub(super) fn draw_setup() -> impl Strategy<Value = sm::Chess> {
        (
            vec![any::<Square>().prop_map_into(); 3],
            select([sm::Role::Knight, sm::Role::Bishop].as_ref()),
            any::<Color>().prop_map_into(),
            any::<u32>(),
            any::<u32>().prop_filter_map("zero", NonZeroU32::new),
        )
            .prop_map(|(s, r, t, hm, fm)| {
                let k = sm::Role::King;

                let pieces = vec![
                    sm::Piece { color: t, role: r },
                    sm::Piece { color: t, role: k },
                    sm::Piece { color: !t, role: k },
                ];

                sm::fen::Fen {
                    board: s.into_iter().zip(pieces).collect(),
                    turn: !t,
                    castling_rights: sm::Bitboard::EMPTY,
                    halfmoves: hm,
                    fullmoves: fm,
                    ..Default::default()
                }
            })
            .prop_filter_map("illegal position", |fen| {
                fen.position(sm::CastlingMode::Standard).ok()
            })
            .prop_filter("stalemate", |setup: &sm::Chess| {
                !sm::Position::is_stalemate(setup)
            })
    }

    fn any_fen() -> impl Strategy<Value = sm::fen::Fen> {
        const CORNERS: &[sm::Square] = &[
            sm::Square::A1,
            sm::Square::A8,
            sm::Square::H1,
            sm::Square::H8,
        ];

        (
            any::<Placement>().prop_map_into(),
            any::<Color>().prop_map_into(),
            hash_set(select(CORNERS), 0..=4).prop_map(|b| b.into_iter().collect()),
            any::<Option<Square>>().prop_map(|s| s.map(sm::Square::from)),
            any::<u32>(),
            any::<u32>().prop_filter_map("zero", NonZeroU32::new),
        )
            .prop_map(|(b, t, cr, eps, hm, fm)| sm::fen::Fen {
                board: b,
                turn: t,
                castling_rights: cr,
                ep_square: eps,
                halfmoves: hm,
                fullmoves: fm,
                ..Default::default()
            })
    }

    fn invalid_fen() -> impl Strategy<Value = (String, sm::fen::ParseFenError)> {
        any_fen()
            .prop_map(|fen| fen.to_string())
            .prop_flat_map(|fen: String| (0..fen.len(), Just(fen), ".+"))
            .prop_map(|(r, fen, s)| [&fen[..r], &s, &fen[r..]].concat())
            .prop_filter_map("valid fen", |s| {
                s.parse::<sm::fen::Fen>().err().map(|e| (s, e))
            })
    }

    fn illegal_fen() -> impl Strategy<Value = (sm::fen::Fen, sm::PositionError<sm::Chess>)> {
        any_fen()
            .prop_filter_map("invalid fen", |fen| fen.to_string().parse().ok())
            .prop_filter_map("legal position", |fen: sm::fen::Fen| {
                fen.position(sm::CastlingMode::Standard)
                    .err()
                    .map(|e| (fen, e))
            })
    }

    fn legal_move(
        setup: impl Strategy<Value = sm::Chess>,
    ) -> impl Strategy<Value = (sm::Chess, sm::uci::Uci)> {
        (setup, any::<Selector>()).prop_filter_map("end position", |(s, selector)| {
            let mvs = sm::Position::legal_moves(&s);
            selector
                .try_select(&mvs)
                .map(sm::uci::Uci::from_standard)
                .map(move |m| (s, m))
        })
    }

    fn illegal_move(
        setup: impl Strategy<Value = sm::Chess>,
    ) -> impl Strategy<Value = (sm::Chess, sm::uci::Uci)> {
        (setup, any::<Move>().prop_map_into()).prop_filter_map("legal move", move |(s, m)| {
            match sm::uci::Uci::to_move(&m, &s) {
                Ok(mv) if !sm::Position::is_legal(&s, &mv) => Some((s, m)),
                Err(_) => Some((s, m)),
                _ => None,
            }
        })
    }

    proptest! {
        #[test]
        fn placement_returns_the_piece_arrangement(pos: Position) {
            let b = sm::fen::Fen::from_setup(&pos.setup).board;
            assert_eq!(pos.placement(), b.into());
        }

        #[test]
        fn turn_returns_the_current_side_to_play(pos: Position) {
            let t = sm::fen::Fen::from_setup(&pos.setup).turn;
            assert_eq!(pos.turn(), t.into());
        }

        #[test]
        fn en_passant_square_returns_the_current_pushed_pawn_skipped_square(pos: Position) {
            let eps = sm::fen::Fen::from_setup(&pos.setup).ep_square;
            assert_eq!(pos.en_passant_square(), eps.map(Into::into));
        }

        #[test]
        fn halfmove_clock_returns_the_number_of_halfmoves_since_last_irreversible_move(pos: Position) {
            let hm = sm::fen::Fen::from_setup(&pos.setup).halfmoves;
            assert_eq!(pos.halfmove_clock(), hm);
        }

        #[test]
        fn fullmoves_returns_the_current_move_number(pos: Position) {
            let fm = sm::fen::Fen::from_setup(&pos.setup).fullmoves;
            assert_eq!(pos.fullmoves(), fm);
        }

        #[test]
        fn is_checkmate_returns_whether_the_position_is_a_checkmate(pos: Checkmate) {
            assert!(pos.is_checkmate());
        }

        #[test]
        fn is_stalemate_returns_whether_the_position_is_a_stalemate(pos: Stalemate) {
            assert!(pos.is_stalemate());
        }

        #[test]
        fn is_draw_returns_whether_the_position_has_insufficient_material(pos: Draw) {
            assert!(pos.is_draw());
        }

        #[test]
        fn moves_returns_the_legal_moves_from_this_position(pos: Position) {
            let mvs: Vec<Move> = sm::Position::legal_moves(&pos.setup)
                .iter()
                .map(sm::uci::Uci::from_standard)
                .map(Into::into)
                .collect();

            assert_eq!(pos.moves().collect::<Vec<_>>(), mvs);
        }

        #[test]
        fn playing_legal_move_updates_position((s, m) in legal_move(any_setup())) {
            let mut pos = Position::from(s.clone());
            let mv = sm::uci::Uci::to_move(&m, &s).unwrap();
            assert_eq!(pos.play(m.into()), Ok(()));
            assert_eq!(pos, sm::Position::play(s, &mv).unwrap().into());
        }

        #[test]
        fn playing_illegal_move_fails((s, m) in illegal_move(any_setup())) {
            let mut pos: Position = s.clone().into();
            assert_eq!(pos.play(m.clone().into()), Err(IllegalMove(m.into(), s.clone().into())));
            assert_eq!(pos, s.into());
        }

        #[test]
        fn parsing_printed_position_is_an_identity(pos: Position) {
            assert_eq!(pos.to_string().parse(), Ok(pos));
        }

        #[test]
        fn parsing_invalid_fen_string_fails((s, e) in invalid_fen()) {
            assert_eq!(s.parse::<Position>(), Err(ParsePositionError::InvalidFen(e.into())));
        }

        #[test]
        fn parsing_illegal_fen_fails((fen, e) in illegal_fen()) {
            assert_eq!(fen.to_string().parse::<Position>(), Err(ParsePositionError::IllegalPosition(e.into())));
        }

        #[test]
        fn position_has_an_equivalent_shakmaty_representation(pos: Position) {
            assert_eq!(Position::from(sm::Chess::from(pos.clone())), pos);
        }
    }
}
