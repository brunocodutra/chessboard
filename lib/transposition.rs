use crate::chess::Move;
use crate::util::{Binary, Bits, Register};
use bitvec::field::BitField;
use derive_more::{Display, Error};
use std::{cmp::Ordering, ops::RangeInclusive};
use test_strategy::Arbitrary;

mod iter;
mod table;

pub use iter::*;
pub use table::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary)]
enum Kind {
    Lower,
    Upper,
    Exact,
}

/// A partial search result.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub struct Transposition {
    kind: Kind,
    score: i16,
    #[strategy(0..=Self::MAX_DEPTH)]
    depth: u8,
    best: Move,
}

impl PartialOrd for Transposition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.depth, self.kind).partial_cmp(&(other.depth, other.kind))
    }
}

impl Ord for Transposition {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.depth, self.kind).cmp(&(other.depth, other.kind))
    }
}

impl Transposition {
    pub const MAX_DEPTH: u8 = (u8::MAX >> 3);

    fn new(kind: Kind, score: i16, depth: u8, best: Move) -> Self {
        assert!(depth <= Self::MAX_DEPTH, "{} <= {}", depth, Self::MAX_DEPTH);

        Transposition {
            kind,
            score,
            depth,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, depth searched, and best [`Move`].
    pub fn lower(score: i16, depth: u8, best: Move) -> Self {
        Transposition::new(Kind::Lower, score, depth, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, depth searched, and best [`Move`].
    pub fn upper(score: i16, depth: u8, best: Move) -> Self {
        Transposition::new(Kind::Upper, score, depth, best)
    }

    /// Constructs a [`Transposition`] given the exact score, depth searched, and best [`Move`].
    pub fn exact(score: i16, depth: u8, best: Move) -> Self {
        Transposition::new(Kind::Exact, score, depth, best)
    }

    /// Bounds for the exact score.
    pub fn bounds(&self) -> RangeInclusive<i16> {
        match self.kind {
            Kind::Lower => self.score..=i16::MAX,
            Kind::Upper => i16::MIN..=self.score,
            Kind::Exact => self.score..=self.score,
        }
    }

    /// Depth searched.
    pub fn depth(&self) -> u8 {
        self.depth
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

type Signature = Bits<u32, 26>;
type OptionalSignedTransposition = Option<(Transposition, Signature)>;
type OptionalSignedTranspositionRegister = <OptionalSignedTransposition as Binary>::Register;

/// The reason why decoding [`Transposition`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "`{}` is not a valid transposition", _0)]
pub struct DecodeTranspositionError(#[error(not(source))] OptionalSignedTranspositionRegister);

impl Binary for OptionalSignedTransposition {
    type Register = Bits<u64, 64>;
    type Error = DecodeTranspositionError;

    fn encode(&self) -> Self::Register {
        match self {
            None => Bits::default(),
            Some((t, sig)) => {
                let mut register = Bits::default();
                let (kind, rest) = register.split_at_mut(2);
                let (score, rest) = rest.split_at_mut(16);
                let (depth, rest) = rest.split_at_mut(5);
                let (best, rest) = rest.split_at_mut(<Move as Binary>::Register::WIDTH);

                kind.store(t.kind as u8);
                score.store(t.score);
                depth.store(t.depth);
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
            let (kind, rest) = register.split_at(2);
            let (score, rest) = rest.split_at(16);
            let (depth, rest) = rest.split_at(5);
            let (best, rest) = rest.split_at(<Move as Binary>::Register::WIDTH);

            use Kind::*;
            Ok(Some((
                Transposition {
                    kind: [Lower, Upper, Exact]
                        .into_iter()
                        .nth(kind.load())
                        .ok_or(DecodeTranspositionError(register))?,
                    score: score.load(),
                    depth: depth.load(),
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
        #[strategy(0..=Transposition::MAX_DEPTH)] d: u8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::lower(s, d, m),
            Transposition::new(Kind::Lower, s, d, m)
        );
    }

    #[proptest]
    fn upper_constructs_upper_bound_transposition(
        s: i16,
        #[strategy(0..=Transposition::MAX_DEPTH)] d: u8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::upper(s, d, m),
            Transposition::new(Kind::Upper, s, d, m)
        );
    }

    #[proptest]
    fn exact_constructs_exact_transposition(
        s: i16,
        #[strategy(0..=Transposition::MAX_DEPTH)] d: u8,
        m: Move,
    ) {
        assert_eq!(
            Transposition::exact(s, d, m),
            Transposition::new(Kind::Exact, s, d, m)
        );
    }

    #[proptest]
    #[should_panic]
    fn transposition_panics_if_depth_grater_than_max(
        k: Kind,
        s: i16,
        #[strategy(Transposition::MAX_DEPTH + 1..)] d: u8,
        m: Move,
    ) {
        Transposition::new(k, s, d, m);
    }

    #[proptest]
    fn transposition_score_is_between_bounds(t: Transposition) {
        assert!(t.bounds().contains(&t.score()));
    }

    #[proptest]
    fn transposition_with_larger_depth_is_larger(
        t: Transposition,
        #[filter(#t.depth != #u.depth)] u: Transposition,
    ) {
        assert_eq!(t < u, t.depth < u.depth);
    }

    #[proptest]
    fn transposition_with_same_depth_is_compared_by_kind(
        t: Transposition,
        #[filter(#t.depth == #u.depth)] u: Transposition,
    ) {
        assert_eq!(t < u, t.kind < u.kind);
    }

    #[proptest]
    fn decoding_encoded_transposition_is_an_identity(t: OptionalSignedTransposition) {
        assert_eq!(Binary::decode(t.encode()), Ok(t));
    }
}
