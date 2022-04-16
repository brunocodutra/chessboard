use crate::{Color, Piece, Role, Square};
use derive_more::{DebugCustom, Display};
use shakmaty as sm;
use std::ops::Index;

/// The arrangement of [`Piece`]s on the chess board.
///
/// This type does not guarantee it holds a valid arrangement of pieces.
#[derive(DebugCustom, Display, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[debug(fmt = "Placement(\"{}\")", self)]
#[display(fmt = "{}", "board")]
pub struct Placement {
    #[cfg_attr(test, proptest(strategy = "tests::any_board()"))]
    board: sm::Board,
}

/// Initializes an empty [`Placement`].
impl Default for Placement {
    fn default() -> Self {
        Placement {
            board: sm::Board::empty(),
        }
    }
}

/// Retrieves the [`Piece`] at a given [`Square`], if any.
impl Index<Square> for Placement {
    type Output = Option<Piece>;

    fn index(&self, s: Square) -> &Self::Output {
        use Color::*;
        use Role::*;
        match self.board.piece_at(s.into()).map(Into::into) {
            Some(Piece(White, Pawn)) => &Some(Piece(White, Pawn)),
            Some(Piece(White, Knight)) => &Some(Piece(White, Knight)),
            Some(Piece(White, Bishop)) => &Some(Piece(White, Bishop)),
            Some(Piece(White, Rook)) => &Some(Piece(White, Rook)),
            Some(Piece(White, Queen)) => &Some(Piece(White, Queen)),
            Some(Piece(White, King)) => &Some(Piece(White, King)),
            Some(Piece(Black, Pawn)) => &Some(Piece(Black, Pawn)),
            Some(Piece(Black, Knight)) => &Some(Piece(Black, Knight)),
            Some(Piece(Black, Bishop)) => &Some(Piece(Black, Bishop)),
            Some(Piece(Black, Rook)) => &Some(Piece(Black, Rook)),
            Some(Piece(Black, Queen)) => &Some(Piece(Black, Queen)),
            Some(Piece(Black, King)) => &Some(Piece(Black, King)),
            None => &None,
        }
    }
}

#[doc(hidden)]
impl From<sm::Board> for Placement {
    fn from(board: sm::Board) -> Self {
        Placement { board }
    }
}

#[doc(hidden)]
impl From<Placement> for sm::Board {
    fn from(p: Placement) -> Self {
        p.board
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{collection::hash_map, prelude::*};

    pub(super) fn any_board() -> impl Strategy<Value = sm::Board> {
        hash_map(
            any::<Square>().prop_map_into(),
            any::<Piece>().prop_map_into(),
            0..=64,
        )
        .prop_map(|b| b.into_iter().collect())
    }

    proptest! {
        #[test]
        fn placement_implements_index_operator(p: Placement, s: Square) {
            assert_eq!(p[s], p.board.piece_at(s.into()).map(Into::into));
        }

        #[test]
        fn placement_is_empty_by_default(s: Square) {
            assert_eq!(Placement::default()[s], None);
        }

        #[test]
        fn placement_has_an_equivalent_shakmaty_representation(p: Placement) {
            assert_eq!(Placement::from(sm::Board::from(p.clone())), p);
        }
    }
}
