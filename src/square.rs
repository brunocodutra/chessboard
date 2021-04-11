use crate::{File, ParseFileError, ParseRankError, Rank};
use derive_more::{Display, Error, From};
use shakmaty as sm;
use std::convert::TryInto;
use std::str::FromStr;
use tracing::instrument;
use vampirc_uci::UciSquare;

/// Denotes a square on the chess board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}", _0, _1)]
pub struct Square(pub File, pub Rank);

impl Square {
    /// This square's [`File`].
    pub fn file(&self) -> File {
        self.0
    }

    /// This square's [`Rank`].
    pub fn rank(&self) -> Rank {
        self.1
    }
}

/// The reason why parsing [`Square`] failed.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error, From)]
#[display(fmt = "unable to parse square; {}")]
pub enum ParseSquareError {
    #[display(fmt = "invalid file")]
    InvalidFile(ParseFileError),
    #[display(fmt = "invalid rank")]
    InvalidRank(ParseRankError),
}

impl FromStr for Square {
    type Err = ParseSquareError;

    #[instrument(level = "trace", err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let i = s.char_indices().nth(1).map_or_else(|| s.len(), |(i, _)| i);
        Ok(Square(s[..i].parse()?, s[i..].parse()?))
    }
}

#[doc(hidden)]
impl From<Square> for UciSquare {
    fn from(s: Square) -> Self {
        UciSquare {
            file: s.file().into(),
            rank: s.rank() as u8,
        }
    }
}

#[doc(hidden)]
impl From<UciSquare> for Square {
    fn from(s: UciSquare) -> Self {
        Square(
            s.file.try_into().unwrap(),
            (s.rank as u32).try_into().unwrap(),
        )
    }
}

#[doc(hidden)]
impl From<Square> for sm::Square {
    fn from(s: Square) -> Self {
        sm::Square::from_coords(s.file().into(), s.rank().into())
    }
}

#[doc(hidden)]
impl From<sm::Square> for Square {
    fn from(s: sm::Square) -> Self {
        Square(s.file().into(), s.rank().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parsing_printed_square_is_an_identity(s: Square) {
            assert_eq!(s.to_string().parse(), Ok(s));
        }

        #[test]
        fn parsing_square_fails_if_file_is_invalid(f in "[^a-h]+", r: Rank) {
            let s = [f, r.to_string()].concat();
            assert_eq!(s.parse::<Square>(), Err(ParseFileError.into()));
        }

        #[test]
        fn parsing_square_fails_if_rank_is_invalid(f: File, r in "[^1-8]*") {
            let s = [f.to_string(), r].concat();
            assert_eq!(s.parse::<Square>(), Err(ParseRankError.into()));
        }

        #[test]
        fn square_has_an_equivalent_vampirc_uci_representation(s: Square) {
            assert_eq!(Square::from(<UciSquare as From<Square>>::from(s)), s);
        }

        #[test]
        fn square_has_an_equivalent_shakmaty_representation(s: Square) {
            assert_eq!(Square::from(sm::Square::from(s)), s);
        }
    }
}
