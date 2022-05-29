use crate::{Color, Piece, Role, Square};
use derive_more::{DebugCustom, Display};
use shakmaty as sm;
use std::ops::Index;

#[cfg(test)]
use proptest::{collection::hash_map, prelude::*};

/// The arrangement of [`Piece`]s on the chess board.
///
/// This type does not guarantee it holds a valid arrangement of pieces.
#[derive(DebugCustom, Display, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "Placement(\"{}\")", self)]
#[display(fmt = "{}", "board")]
pub struct Placement {
    #[cfg_attr(test, strategy(
        hash_map(any::<Square>().prop_map_into(), any::<Piece>().prop_map_into(), 0..=64)
            .prop_map(|b| b.into_iter().collect())
    ))]
    board: sm::Board,
}

impl Placement {
    /// The number of pieces of a [`Color`].
    pub fn material(&self, c: Color) -> usize {
        self.board.by_color(c.into()).count()
    }

    /// The number of pieces of a kind.
    pub fn pieces(&self, p: Piece) -> usize {
        self.board.by_piece(p.into()).count()
    }
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

#[doc(hidden)]
impl AsRef<sm::Board> for Placement {
    fn as_ref(&self) -> &sm::Board {
        &self.board
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn placement_implements_index_operator(p: Placement, s: Square) {
        assert_eq!(p[s], p.board.piece_at(s.into()).map(Into::into));
    }

    #[proptest]
    fn placement_is_empty_by_default(s: Square) {
        assert_eq!(Placement::default()[s], None);
    }

    #[proptest]
    fn material_counts_number_of_pieces_of_a_color(p: Placement, c: Color) {
        assert_eq!(
            p.material(c),
            (p.board.occupied() & !p.board.by_color((!c).into())).count()
        );
    }

    #[proptest]
    fn pieces_counts_number_of_pieces_of_a_kind(p: Placement, pc: Piece) {
        assert_eq!(
            p.pieces(pc),
            (p.board.by_color(pc.0.into()) & p.board.by_role(pc.1.into())).count()
        );
    }

    #[proptest]
    fn placement_has_an_equivalent_shakmaty_representation(p: Placement) {
        assert_eq!(Placement::from(sm::Board::from(p.clone())), p);
    }
}
