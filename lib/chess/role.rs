use cozy_chess as cc;
use derive_more::Display;

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

impl Role {
    pub const ALL: [Self; 6] = [
        Role::Pawn,
        Role::Knight,
        Role::Bishop,
        Role::Rook,
        Role::Queen,
        Role::King,
    ];

    /// Constructs [`Role`] from index.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not in the range (0..=5).
    pub fn from_index(i: u8) -> Self {
        Self::ALL[i as usize]
    }

    /// This role's index in the range (0..=5).
    pub fn index(&self) -> u8 {
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
        Role::from_index(r as _)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Buffer;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn role_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Role>>(), size_of::<Role>());
    }

    #[proptest]
    fn role_has_an_index(r: Role) {
        assert_eq!(Role::from_index(r.index()), r);
    }

    #[proptest]

    fn from_index_constructs_role_by_index(#[strategy(0u8..6)] i: u8) {
        assert_eq!(Role::from_index(i).index(), i);
    }

    #[proptest]
    #[should_panic]

    fn from_index_panics_if_index_out_of_range(#[strategy(6u8..)] i: u8) {
        Role::from_index(i);
    }

    #[proptest]
    fn role_is_ordered_by_index(a: Role, b: Role) {
        assert_eq!(a < b, a.index() < b.index());
    }

    #[proptest]
    fn all_contains_roles_in_order() {
        assert_eq!(
            Role::ALL.into_iter().collect::<Buffer<_, 6>>(),
            (0..6).map(Role::from_index).collect()
        );
    }

    #[proptest]
    fn role_has_an_equivalent_cozy_chess_representation(r: Role) {
        assert_eq!(Role::from(cc::Piece::from(r)), r);
    }
}
