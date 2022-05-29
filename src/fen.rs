use crate::Position;
use derive_more::{DebugCustom, Display, Error};
use shakmaty as sm;
use std::str::FromStr;

#[cfg(test)]
use proptest::prelude::*;

/// A representation of the [Forsyth–Edwards Notation].
///
/// [Forsyth–Edwards Notation]: https://en.wikipedia.org/wiki/Forsyth%E2%80%93Edwards_Notation
#[derive(DebugCustom, Display, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "Fen(\"{}\")", self)]
#[display(fmt = "{}", "setup")]
pub struct Fen {
    #[cfg_attr(test, strategy(
        any::<crate::Placement>().prop_filter_map("invalid fen", |p| {
            let fen = sm::fen::Fen(sm::Setup { board: p.into(), ..Default::default() });
            fen.to_string().parse().ok()
        })
    ))]
    setup: sm::fen::Fen,
}

/// The reason why the string is not valid FEN.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
pub enum ParseFenError {
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
impl From<sm::fen::ParseFenError> for ParseFenError {
    fn from(e: sm::fen::ParseFenError) -> Self {
        use ParseFenError::*;
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

impl FromStr for Fen {
    type Err = ParseFenError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Fen { setup: s.parse()? })
    }
}

impl From<Position> for Fen {
    fn from(pos: Position) -> Self {
        sm::Setup::from(pos).into()
    }
}

#[doc(hidden)]
impl From<sm::Setup> for Fen {
    fn from(setup: sm::Setup) -> Self {
        Fen {
            setup: sm::fen::Fen(setup),
        }
    }
}

#[doc(hidden)]
impl From<Fen> for sm::Setup {
    fn from(fen: Fen) -> Self {
        fen.setup.into()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_fen_is_an_identity(fen: Fen) {
        assert_eq!(fen.to_string().parse(), Ok(fen));
    }

    #[proptest]
    fn parsing_invalid_fen_fails(
        #[by_ref] fen: Fen,
        #[strategy(..=#fen.to_string().len())] n: usize,
        #[strategy("[^[:ascii:]]+")] r: String,
    ) {
        assert!([&fen.to_string()[..n], &r].concat().parse::<Fen>().is_err());
    }
}
