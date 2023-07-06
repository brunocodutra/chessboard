use shakmaty as sm;
use test_strategy::Arbitrary;

/// Denotes the type of a chess [`Piece`][`crate::Piece`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
pub enum Role {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Role {
    /// Returns an iterator over [`Role`]s.
    #[inline]
    pub fn iter() -> impl DoubleEndedIterator<Item = Self> + ExactSizeIterator {
        sm::Role::ALL.into_iter().map(Role::from)
    }
}

#[doc(hidden)]
impl From<Role> for sm::Role {
    #[inline]
    fn from(r: Role) -> Self {
        match r {
            Role::Pawn => sm::Role::Pawn,
            Role::Knight => sm::Role::Knight,
            Role::Bishop => sm::Role::Bishop,
            Role::Rook => sm::Role::Rook,
            Role::Queen => sm::Role::Queen,
            Role::King => sm::Role::King,
        }
    }
}

#[doc(hidden)]
impl From<sm::Role> for Role {
    #[inline]
    fn from(r: sm::Role) -> Self {
        match r {
            sm::Role::Pawn => Role::Pawn,
            sm::Role::Knight => Role::Knight,
            sm::Role::Bishop => Role::Bishop,
            sm::Role::Rook => Role::Rook,
            sm::Role::Queen => Role::Queen,
            sm::Role::King => Role::King,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn role_has_an_equivalent_shakmaty_representation(r: Role) {
        assert_eq!(Role::from(sm::Role::from(r)), r);
    }
}
