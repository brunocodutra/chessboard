use crate::chess::{Color, Perspective, Piece, Square};
use crate::util::Integer;

/// The HalfKAv2 feature.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Feature(pub Square, pub Piece, pub Square);

impl Feature {
    /// Feature's index for the given perspective.
    #[inline(always)]
    pub fn index(&self, side: Color) -> u16 {
        let Feature(ks, p, s) = self.perspective(side);
        s as u16 + 64 * (p.repr().min(10) as u16 + 11 * ks as u16)
    }
}

impl const Perspective for Feature {
    #[inline(always)]
    fn flip(&self) -> Self {
        Feature(self.0.flip(), self.1.flip(), self.2.flip())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn feature_has_a_unique_index(a: Feature, #[filter(#a != #b)] b: Feature, c: Color) {
        assert_ne!(a.index(c), b.index(c));
    }

    #[proptest]
    fn feature_index_is_unique_to_perspective(a: Feature, c: Color) {
        assert_ne!(a.index(c), a.index(!c));
    }

    #[proptest]
    fn feature_has_a_mirror(a: Feature) {
        assert_ne!(a.flip(), a);
        assert_eq!(a.flip().flip(), a);
    }
}
