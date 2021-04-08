use derive_more::Display;
use shakmaty as sm;

/// Denotes the type of a chess [`Piece`][`crate::Piece`].
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Role {
    #[display(fmt = "pawn")]
    Pawn,
    #[display(fmt = "knight")]
    Knight,
    #[display(fmt = "bishop")]
    Bishop,
    #[display(fmt = "rook")]
    Rook,
    #[display(fmt = "queen")]
    Queen,
    #[display(fmt = "king")]
    King,
}

#[doc(hidden)]
impl From<Role> for sm::Role {
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
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn role_has_an_equivalent_shakmaty_representation(r: Role) {
            assert_eq!(Role::from(sm::Role::from(r)), r);
        }
    }
}
