use crate::{file::*, foreign, rank::*};
use derive_more::Display;

/// A square of the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}", "self.file.to_str()", "self.rank.to_str()")]
pub struct Square {
    pub file: File,
    pub rank: Rank,
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
