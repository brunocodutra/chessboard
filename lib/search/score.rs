use crate::util::{Binary, Bits, Integer, Saturating};
use crate::{chess::Perspective, search::Ply};
use std::fmt;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct ScoreRepr(#[cfg_attr(test, strategy(Self::MIN..=Self::MAX))] <Self as Integer>::Repr);

unsafe impl const Integer for ScoreRepr {
    type Repr = i16;
    const MIN: Self::Repr = -Self::MAX;
    const MAX: Self::Repr = 8191;
}

/// The minimax score.
pub type Score = Saturating<ScoreRepr>;

impl Score {
    /// Returns number of plies to mate, if one is in the horizon.
    ///
    /// Negative number of plies means the opponent is mating.
    pub fn mate(&self) -> Option<Ply> {
        if *self <= Score::lower() - Ply::MIN {
            Some((Score::lower() - *self).saturate())
        } else if *self >= Score::upper() - Ply::MAX {
            Some((Score::upper() - *self).saturate())
        } else {
            None
        }
    }

    /// Normalizes mate scores relative to `ply`.
    pub fn normalize(&self, ply: Ply) -> Self {
        if *self <= Score::lower() - Ply::MIN {
            (*self + ply).min(Score::lower() - Ply::MIN)
        } else if *self >= Score::upper() - Ply::MAX {
            (*self - ply).max(Score::upper() - Ply::MAX)
        } else {
            *self
        }
    }
}

impl const Perspective for Score {
    #[inline(always)]
    fn flip(&self) -> Self {
        Self::new(-self.get())
    }
}

impl Binary for Score {
    type Bits = Bits<u16, 14>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        Bits::new((self.get() - Self::lower().get()).cast())
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        Self::lower() + bits.cast::<i16>()
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mate() {
            Some(p) if p > 0 => write!(f, "{:+}#{}", self.get(), (p.get() + 1) / 2),
            Some(p) => write!(f, "{:+}#{}", self.get(), (1 - p.get()) / 2),
            None => write!(f, "{:+}", self.get()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn normalize_ignores_non_mate_scores(#[filter(#s.mate().is_none())] s: Score, p: Ply) {
        assert_eq!(s.normalize(p), s);
    }

    #[proptest]
    fn normalize_preserves_mate_score(s: Score, p: Ply) {
        assert_eq!(s.normalize(p).mate().is_some(), s.mate().is_some());
    }

    #[proptest]
    fn mate_returns_plies_to_mate(p: Ply) {
        if p > 0 {
            assert_eq!(Score::upper().normalize(p).mate(), Some(p));
        } else {
            assert_eq!(Score::lower().normalize(-p).mate(), Some(p));
        }
    }

    #[proptest]
    fn decoding_encoded_score_is_an_identity(s: Score) {
        assert_eq!(Score::decode(s.encode()), s);
    }
}
