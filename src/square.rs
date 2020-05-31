use crate::{foreign, File, ParseFileError, ParseRankError, Rank};
use derive_more::{Display, Error, From};
use std::str::{self, FromStr};

/// A square of the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}", "self.file", "self.rank")]
pub struct Square {
    pub file: File,
    pub rank: Rank,
}

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error, From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "unable to parse square, {}")]
pub enum ParseSquareError {
    #[display(fmt = "invalid file")]
    InvalidFile(ParseFileError),
    #[display(fmt = "invalid rank")]
    InvalidRank(ParseRankError),
}

impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (i, _) = s.char_indices().nth(1).unwrap_or((s.len(), '\0'));

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
    fn into(self: Self) -> foreign::Square {
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
        fn parsing_square_fails_if_file_is_invalid(s in "[^a-h]*[1-8]") {
            assert_eq!(s.parse::<Square>(), Err(ParseSquareError::InvalidFile(ParseFileError)));
        }

        #[test]
        fn parsing_square_fails_if_rank_is_invalid(s in "[a-h][^1-8]*") {
            assert_eq!(s.parse::<Square>(), Err(ParseSquareError::InvalidRank(ParseRankError)));
        }
    }
}
