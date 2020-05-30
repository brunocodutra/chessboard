use crate::{figure::*, file::*, foreign, rank::*, square::*};
use std::{fmt, ops::*};

/// A position on the board.
///
/// This type does not validate whether the position it holds is valid
/// according to any set of chess rules.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Position {
    pub squares: [[Option<Figure>; 8]; 8],
}

// We provide a custom implementation of Arbitrary rather than deriving,
// otherwise proptest overflows the stack generating large arrays.
#[cfg(test)]
impl proptest::arbitrary::Arbitrary for Position {
    type Parameters = ();
    type Strategy = proptest::prelude::BoxedStrategy<Position>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        vec![any::<Option<Figure>>(); 64]
            .prop_map(|v| {
                let mut squares: [[Option<Figure>; 8]; 8] = Default::default();
                squares
                    .iter_mut()
                    .flatten()
                    .zip(v)
                    .for_each(|(s, f)| *s = f);
                Position { squares }
            })
            .boxed()
    }
}

impl Index<Square> for Position {
    type Output = Option<Figure>;

    fn index(&self, s: Square) -> &Self::Output {
        &self.squares[s.rank as usize][s.file as usize]
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "   ")?;

        for &file in File::VARIANTS {
            write!(f, "  {} ", file)?;
        }

        writeln!(f)?;
        writeln!(f, "   +---+---+---+---+---+---+---+---+")?;
        for (&rank, row) in Rank::VARIANTS.iter().zip(&self.squares).rev() {
            write!(f, " {} |", rank)?;

            for &figure in row {
                match figure {
                    Some(figure) => write!(f, " {} |", figure)?,
                    None => write!(f, "   |",)?,
                }
            }
            writeln!(f, " {}", rank)?;
            writeln!(f, "   +---+---+---+---+---+---+---+---+")?;
        }

        write!(f, "   ")?;
        for &file in File::VARIANTS {
            write!(f, "  {} ", file)?;
        }

        Ok(())
    }
}

impl From<foreign::Board> for Position {
    fn from(b: foreign::Board) -> Self {
        let mut squares: [[Option<Figure>; 8]; 8] = Default::default();

        for &s in foreign::ALL_SQUARES.iter() {
            squares[s.get_rank().to_index()][s.get_file().to_index()] =
                b.piece_on(s).and_then(|p| {
                    b.color_on(s).map(move |c| Figure {
                        piece: p.into(),
                        color: c.into(),
                    })
                });
        }

        Position { squares }
    }
}
