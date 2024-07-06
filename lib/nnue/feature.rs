use crate::chess::{Color, Perspective, Piece, Square};
use crate::util::Integer;

/// The HalfKAv2 feature.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct Feature(#[cfg_attr(test, strategy(Self::MIN..=Self::MAX))] <Self as Integer>::Repr);

unsafe impl const Integer for Feature {
    type Repr = u16;
    const MIN: Self::Repr = 0;
    const MAX: Self::Repr = Self::LEN as Self::Repr - 1;
}

impl Feature {
    /// The total number of different features.
    pub const LEN: usize = 64 * 704;

    /// Constructs feature from some perspective.
    #[inline(always)]
    pub fn new(side: Color, ksq: Square, piece: Piece, sq: Square) -> Self {
        let psq = sq.perspective(side) as u16 + 64 * piece.perspective(side).get().min(10) as u16;
        Feature(psq + 704 * ksq.perspective(side) as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[test]
    fn len_counts_total_number_of_features() {
        assert_eq!(Feature::LEN, Feature::iter().len());
    }

    #[proptest]
    fn is_unique_to_perspective(c: Color, ksq: Square, p: Piece, sq: Square) {
        assert_ne!(Feature::new(c, ksq, p, sq), Feature::new(!c, ksq, p, sq));
    }
}
