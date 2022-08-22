use crate::{Binary, Bits, Move, Register};
use bitvec::field::BitField;
use derive_more::{Display, Error};
use std::ops::RangeInclusive;

mod iter;
mod table;

pub use iter::*;
pub use table::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
enum TranspositionKind {
    Lower,
    Upper,
}

/// A partial search result.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Transposition {
    kind: TranspositionKind,
    score: i16,
    #[cfg_attr(test, strategy(Self::MIN_DRAFT..=Self::MAX_DRAFT))]
    draft: i8,
    best: Move,
}

impl Transposition {
    pub const MIN_DRAFT: i8 = (i8::MIN >> 1);
    pub const MAX_DRAFT: i8 = (i8::MAX >> 1);

    fn new(kind: TranspositionKind, score: i16, draft: i8, best: Move) -> Self {
        assert!(draft >= Self::MIN_DRAFT, "{} >= {}", draft, Self::MIN_DRAFT);
        assert!(draft <= Self::MAX_DRAFT, "{} <= {}", draft, Self::MAX_DRAFT);

        Transposition {
            kind,
            score,
            draft,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, remaining draft, and best [`Move`].
    pub fn lower(score: i16, draft: i8, best: Move) -> Self {
        Transposition::new(TranspositionKind::Lower, score, draft, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, remaining draft, and best [`Move`].
    pub fn upper(score: i16, draft: i8, best: Move) -> Self {
        Transposition::new(TranspositionKind::Upper, score, draft, best)
    }

    /// Bounds for the exact score.
    pub fn bounds(&self) -> RangeInclusive<i16> {
        match self.kind {
            TranspositionKind::Lower => self.score..=i16::MAX,
            TranspositionKind::Upper => i16::MIN..=self.score,
        }
    }

    /// Remaining draft.
    pub fn draft(&self) -> i8 {
        self.draft
    }

    /// Partial score.
    pub fn score(&self) -> i16 {
        self.score
    }

    /// Best [`Move`] at this depth.
    pub fn best(&self) -> Move {
        self.best
    }
}

type Signature = Bits<u32, 25>;
type OptionalSignedTransposition = Option<(Transposition, Signature)>;
type OptionalSignedTranspositionRegister = <OptionalSignedTransposition as Binary>::Register;

/// The reason why decoding [`Transposition`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "`{}` is not a valid search result", _0)]
pub struct DecodeTranspositionError(#[error(not(source))] OptionalSignedTranspositionRegister);

impl Binary for OptionalSignedTransposition {
    type Register = Bits<u64, 64>;
    type Error = DecodeTranspositionError;

    fn encode(&self) -> Self::Register {
        match self {
            None => Bits::default(),
            Some((t, sig)) => {
                let mut register = Bits::default();
                let (kind, rest) = register.split_at_mut(1);
                let (score, rest) = rest.split_at_mut(16);
                let (draft, rest) = rest.split_at_mut(7);
                let (best, rest) = rest.split_at_mut(<Move as Binary>::Register::WIDTH);

                kind.store(t.kind as u8);
                score.store(t.score);
                draft.store(t.draft);
                best.clone_from_bitslice(&t.best.encode());
                rest.clone_from_bitslice(sig);

                debug_assert_ne!(register, Bits::default());

                register
            }
        }
    }

    fn decode(register: Self::Register) -> Result<Self, Self::Error> {
        if register == Bits::default() {
            Ok(None)
        } else {
            let (kind, rest) = register.split_at(1);
            let (score, rest) = rest.split_at(16);
            let (draft, rest) = rest.split_at(7);
            let (best, rest) = rest.split_at(<Move as Binary>::Register::WIDTH);

            use TranspositionKind::*;
            Ok(Some((
                Transposition {
                    kind: [Lower, Upper][kind.load::<usize>()],
                    score: score.load(),
                    draft: draft.load(),
                    best: Binary::decode(best.into())
                        .map_err(|_| DecodeTranspositionError(register))?,
                },
                rest.into(),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn lower_constructs_lower_bound_transposition(
        s: i16,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::lower(s, d, m),
            Transposition::new(TranspositionKind::Lower, s, d, m)
        );
    }

    #[proptest]
    fn upper_constructs_lower_bound_transposition(
        s: i16,
        #[strategy(Transposition::MIN_DRAFT..=Transposition::MAX_DRAFT)] d: i8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::upper(s, d, m),
            Transposition::new(TranspositionKind::Upper, s, d, m)
        );
    }

    #[proptest]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn transposition_panics_if_draft_grater_than_max(
        k: TranspositionKind,
        s: i16,
        #[strategy(Transposition::MAX_DRAFT + 1..)] d: i8,
        m: Move,
    ) {
        Transposition::new(k, s, d, m);
    }

    #[proptest]
    #[cfg(debug_assertions)]
    #[should_panic]
    fn transposition_panics_if_draft_lower_than_max(
        k: TranspositionKind,
        s: i16,
        #[strategy(..Transposition::MIN_DRAFT - 1)] d: i8,
        m: Move,
    ) {
        Transposition::new(k, s, d, m);
    }

    #[proptest]
    fn transposition_score_is_between_bounds(t: Transposition) {
        assert!(t.bounds().contains(&t.score()));
    }

    #[proptest]
    fn decoding_encoded_transposition_is_an_identity(t: OptionalSignedTransposition) {
        assert_eq!(Binary::decode(t.encode()), Ok(t));
    }
}
