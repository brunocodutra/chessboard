use crate::util::Enum;
use cozy_chess as cc;
use derive_more::Display;
use std::ops::RangeInclusive;

/// The type of a chess [`Piece`][`crate::Piece`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
pub enum Role {
    #[display("p")]
    Pawn,
    #[display("n")]
    Knight,
    #[display("b")]
    Bishop,
    #[display("r")]
    Rook,
    #[display("q")]
    Queen,
    #[display("k")]
    King,
}

unsafe impl Enum for Role {
    const RANGE: RangeInclusive<Self> = Role::Pawn..=Role::King;

    #[inline(always)]
    fn repr(&self) -> u8 {
        *self as _
    }
}

#[doc(hidden)]
impl From<Role> for cc::Piece {
    #[inline(always)]
    fn from(r: Role) -> Self {
        cc::Piece::index_const(r as _)
    }
}

#[doc(hidden)]
impl From<cc::Piece> for Role {
    #[inline(always)]
    fn from(r: cc::Piece) -> Self {
        Role::from_repr(r as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn role_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Role>>(), size_of::<Role>());
    }

    #[proptest]
    fn role_has_an_equivalent_cozy_chess_representation(r: Role) {
        assert_eq!(Role::from(cc::Piece::from(r)), r);
    }
}
