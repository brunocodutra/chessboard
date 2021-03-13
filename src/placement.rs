use crate::{File, Piece, Rank, Square};
use std::ops::{Index, IndexMut};

/// The piece placement on the board.
///
/// This type does not validate whether the placement it holds is valid
/// according to any set of chess rules.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Placement {
    pub squares: [[Option<Piece>; 8]; 8],
}

impl Index<Square> for Placement {
    type Output = Option<Piece>;

    fn index(&self, s: Square) -> &Self::Output {
        &self.squares[s.rank as usize - Rank::First as usize][s.file as usize - File::A as usize]
    }
}

impl IndexMut<Square> for Placement {
    fn index_mut(&mut self, s: Square) -> &mut Self::Output {
        &mut self.squares[s.rank as usize - Rank::First as usize]
            [s.file as usize - File::A as usize]
    }
}
