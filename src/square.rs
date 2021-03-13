use crate::{foreign, File, ParseFileError, ParseRankError, Rank};
use derive_more::{Display, Error, From};
use std::str::{self, FromStr};
use tracing::instrument;

/// A square of the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}", file, rank)]
pub struct Square {
    pub file: File,
    pub rank: Rank,
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

    #[instrument(err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let i = s.char_indices().nth(1).map_or_else(|| s.len(), |(i, _)| i);

        Ok(Square {
            file: s[..i].parse()?,
            rank: s[i..].parse()?,
        })
    }
}

impl From<foreign::Square> for Square {
    fn from(s: foreign::Square) -> Self {
        Square {
            file: s.get_file().into(),
            rank: s.get_rank().into(),
        }
    }
}

impl Into<foreign::Square> for Square {
    fn into(self) -> foreign::Square {
        foreign::Square::make_square(self.rank.into(), self.file.into())
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
    }
}
