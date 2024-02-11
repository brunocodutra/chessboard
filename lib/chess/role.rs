use crate::util::Integer;
use derive_more::{Display, Error};
use std::str::FromStr;

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

unsafe impl const Integer for Role {
    type Repr = u8;
    const MIN: Self::Repr = Role::Pawn as _;
    const MAX: Self::Repr = Role::King as _;
}

/// The reason why parsing the piece.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    "failed to parse piece, expected one of `[{}{}{}{}{}{}]`",
    Role::Pawn,
    Role::Knight,
    Role::Bishop,
    Role::Rook,
    Role::Queen,
    Role::King
)]
pub struct ParseRoleError;

impl FromStr for Role {
    type Err = ParseRoleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "p" => Ok(Role::Pawn),
            "n" => Ok(Role::Knight),
            "b" => Ok(Role::Bishop),
            "r" => Ok(Role::Rook),
            "q" => Ok(Role::Queen),
            "k" => Ok(Role::King),
            _ => Err(ParseRoleError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[test]
    fn role_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Role>>(), size_of::<Role>());
    }

    #[proptest]
    fn parsing_printed_role_is_an_identity(r: Role) {
        assert_eq!(r.to_string().parse(), Ok(r));
    }

    #[proptest]
    fn parsing_role_fails_if_not_one_of_lowercase_pnbrqk(
        #[filter(!['p', 'n', 'b', 'r', 'q', 'k'].contains(&#c))] c: char,
    ) {
        assert_eq!(c.to_string().parse::<Role>(), Err(ParseRoleError));
    }

    #[proptest]
    fn parsing_role_fails_if_length_not_one(#[filter(#s.len() != 1)] s: String) {
        assert_eq!(s.parse::<Role>(), Err(ParseRoleError));
    }
}
