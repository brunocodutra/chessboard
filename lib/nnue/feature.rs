use crate::chess::{Color, Piece, Square};

/// The HalfKAv2 feature.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Feature(pub Square, pub Piece, pub Square);

impl Feature {
    /// Mirrors this feature.
    pub fn mirror(&self) -> Self {
        Feature(self.0.mirror(), self.1.mirror(), self.2.mirror())
    }

    /// Feature's index for the given perspective.
    pub fn index(&self, side: Color) -> u16 {
        let Feature(ks, p, s) = match side {
            Color::White => *self,
            Color::Black => self.mirror(),
        };

        s.index() as u16 + 64 * (p.index().min(10) as u16 + 11 * ks.index() as u16)
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
        assert_ne!(a.mirror(), a);
        assert_eq!(a.mirror().mirror(), a);
    }
}
