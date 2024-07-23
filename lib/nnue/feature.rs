use crate::chess::{Color, File, Mirror, Perspective, Piece, Square};
use crate::util::Integer;

/// The HalfKAv2 feature.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct Feature(#[cfg_attr(test, strategy(Self::MIN..=Self::MAX))] <Self as Integer>::Repr);

unsafe impl Integer for Feature {
    type Repr = u16;
    const MIN: Self::Repr = 0;
    const MAX: Self::Repr = Self::LEN as Self::Repr - 1;
}

impl Feature {
    /// The total number of different features.
    pub const LEN: usize = 8 * 768;

    #[rustfmt::skip]
    const KING_BUCKETS: [u16; 64] = [
        0, 0, 1, 1, 1, 1, 0, 0,
        2, 2, 3, 3, 3, 3, 2, 2,
        4, 4, 5, 5, 5, 5, 4, 4,
        4, 4, 5, 5, 5, 5, 4, 4,
        6, 6, 7, 7, 7, 7, 6, 6,
        6, 6, 7, 7, 7, 7, 6, 6,
        6, 6, 7, 7, 7, 7, 6, 6,
        6, 6, 7, 7, 7, 7, 6, 6,
    ];

    /// Constructs feature from some perspective.
    #[inline(always)]
    pub fn new(side: Color, ksq: Square, piece: Piece, sq: Square) -> Self {
        let psq = 64 * piece.perspective(side) as u16
            + if ksq.file() <= File::D {
                sq.perspective(side).mirror() as u16
            } else {
                sq.perspective(side) as u16
            };

        Feature(psq + 768 * Self::KING_BUCKETS[ksq.perspective(side) as usize])
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
