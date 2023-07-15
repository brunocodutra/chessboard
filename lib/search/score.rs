use crate::search::Ply;
use crate::util::{Binary, Bits, Bounds, Saturating};
use derive_more::{Display, Error};
use std::fmt;
use test_strategy::Arbitrary;

pub struct ScoreBounds;

impl Bounds for ScoreBounds {
    type Integer = i16;
    const LOWER: Self::Integer = -Self::UPPER;
    const UPPER: Self::Integer = 16383;
}

/// The minimax score.
pub type Score = Saturating<ScoreBounds>;

impl Score {
    /// Returns number of plies to mate, if one is in the horizon.
    ///
    /// Negative number of plies means the opponent is mating.
    pub fn mate(&self) -> Option<Ply> {
        if *self <= Score::lower() - Ply::lower() {
            Some((Score::lower() - *self).cast())
        } else if *self >= Score::upper() - Ply::upper() {
            Some((Score::upper() - *self).cast())
        } else {
            None
        }
    }

    /// Normalizes mate scores relative to `ply`.
    pub fn normalize(&self, ply: Ply) -> Self {
        if *self <= Score::lower() - Ply::lower() {
            (*self + ply).min(Score::lower() - Ply::lower())
        } else if *self >= Score::upper() - Ply::upper() {
            (*self - ply).max(Score::upper() - Ply::upper())
        } else {
            *self
        }
    }
}

/// The reason why decoding [`Score`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid score")]
pub struct DecodeScoreError;

impl Binary for Score {
    type Bits = Bits<u16, 15>;
    type Error = DecodeScoreError;

    fn encode(&self) -> Self::Bits {
        Bits::new((self.get() - ScoreBounds::LOWER) as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        if bits != !Bits::default() {
            Ok(Self::lower() + bits.get())
        } else {
            Err(DecodeScoreError)
        }
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
    use crate::search::Value;
    use test_strategy::proptest;

    #[proptest]
    fn normalize_ignores_non_mate_scores(v: Value, p: Ply) {
        let s: Score = v.cast();
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
        assert_eq!(Binary::decode(s.encode()), Ok(s));
    }

    #[proptest]
    fn decoding_value_fails_for_invalid_bits() {
        let b = !<Score as Binary>::Bits::default();
        assert_eq!(Score::decode(b), Err(DecodeScoreError));
    }
}
