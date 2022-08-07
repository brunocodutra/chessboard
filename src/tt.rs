use crate::{Binary, Bits, Cache, Move, Register};
use bitvec::{field::BitField, mem::BitRegister, store::BitStore};
use derive_more::{Display, Error};
use std::ops::RangeInclusive;

#[cfg(test)]
use proptest::prelude::*;

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
    pub const MIN_DRAFT: i8 = (i8::MIN >> 1) + 32;
    pub const MAX_DRAFT: i8 = (i8::MAX >> 1) + 32;

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

type Key = Bits<u64, 64>;
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
                draft.store(t.draft - 32);
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
                    draft: draft.load::<i8>() + 32,
                    best: Binary::decode(best.into())
                        .map_err(|_| DecodeTranspositionError(register))?,
                },
                rest.into(),
            )))
        }
    }
}

/// A cache for [`Transposition`]s.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = <Cache<OptionalSignedTransposition> as Arbitrary>::Parameters))]
pub struct TranspositionTable {
    #[cfg_attr(test, any((*args).clone()))]
    cache: Cache<OptionalSignedTranspositionRegister>,
}

impl TranspositionTable {
    /// Constructs a [`TranspositionTable`] of at most `size` many bytes.
    ///
    /// The `size` specifies an upper bound.
    pub fn new(size: usize) -> Self {
        let entry_size = OptionalSignedTranspositionRegister::SIZE;
        let cache_size = (size / entry_size + 1).next_power_of_two() / 2;

        TranspositionTable {
            cache: Cache::new(cache_size),
        }
    }

    /// The actual size of this [`TranspositionTable`] in bytes.
    pub fn size(&self) -> usize {
        self.len() * OptionalSignedTranspositionRegister::SIZE
    }

    /// The actual size of this [`TranspositionTable`] in number of entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the [`TranspositionTable`] is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    fn index_of<T, const N: usize>(&self, key: Bits<T, N>) -> usize
    where
        T: BitStore + BitRegister,
    {
        match self.len().trailing_zeros() as usize {
            0 => 0,
            w => key[..w].load(),
        }
    }

    /// Loads the [`Transposition`] from the slot associated with `key`.
    pub fn get(&self, key: Key) -> Option<Transposition> {
        if self.is_empty() {
            None
        } else {
            let sig = key[(Key::WIDTH - Signature::WIDTH)..].into();
            let register = self.cache.load(self.index_of(key));
            match Binary::decode(register).expect("expected valid encoding") {
                Some((t, s)) if s == sig => Some(t),
                _ => None,
            }
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Ordering::Greater`] [`Transposition`] is chosen.
    pub fn set(&self, key: Key, transposition: Transposition) {
        if !self.is_empty() {
            self.cache.update(self.index_of(key), |r| {
                match Binary::decode(r).expect("expected valid encoding") {
                    Some((t, _)) if t.draft() > transposition.draft() => None,
                    _ => Some((transposition, key[(Key::WIDTH - Signature::WIDTH)..].into()))
                        .encode()
                        .into(),
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::view::BitView;
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
    fn panics_for_draft_grater_than_max(
        k: TranspositionKind,
        s: i16,
        #[strategy((Transposition::MAX_DRAFT + 1)..)] d: i8,
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

    #[proptest]
    fn size_returns_table_capacity_in_bytes(tt: TranspositionTable) {
        assert_eq!(
            tt.size(),
            tt.cache.len() * OptionalSignedTranspositionRegister::SIZE
        );
    }

    #[proptest]
    fn input_size_is_an_upper_limit(#[strategy(0usize..1024)] s: usize) {
        assert!(TranspositionTable::new(s).size() <= s);
    }

    #[proptest]
    fn size_is_exact_if_input_is_power_of_two(#[strategy(3usize..=10)] w: usize) {
        assert_eq!(TranspositionTable::new(1 << w).size(), 1 << w);
    }

    #[proptest]
    fn len_returns_table_capacity(tt: TranspositionTable) {
        assert_eq!(tt.len(), tt.cache.len());
    }

    #[proptest]
    fn get_returns_none_if_table_is_empty(k: Key) {
        let tt = TranspositionTable::new(0);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_transposition_does_not_exist(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        k: Key,
    ) {
        tt.cache.store(tt.index_of(k), Bits::default());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_signature_does_not_match(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        t: Transposition,
        k: Key,
    ) {
        let sig = (!k.load::<u64>()).view_bits()[(Key::WIDTH - Signature::WIDTH)..].into();
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        t: Transposition,
        k: Key,
    ) {
        let sig = k[(Key::WIDTH - Signature::WIDTH)..].into();
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_does_nothing_if_table_is_empty(t: Transposition, k: Key) {
        let tt = TranspositionTable::new(0);
        tt.set(k, t);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn set_keeps_transposition_with_larger_draft(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        t: Transposition,
        u: Transposition,
        k: Key,
    ) {
        let sig = k[(Key::WIDTH - Signature::WIDTH)..].into();
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        tt.set(k, u);

        if t.draft() > u.draft() {
            assert_eq!(tt.get(k), Some(t));
        } else {
            assert_eq!(tt.get(k), Some(u));
        }
    }

    #[proptest]
    fn set_ignores_the_signature_mismatch(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        t: Transposition,
        #[filter(#u.draft() > #t.draft())] u: Transposition,
        k: Key,
    ) {
        let sig = (!k.load::<u64>()).view_bits()[(Key::WIDTH - Signature::WIDTH)..].into();
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        tt.set(k, u);
        assert_eq!(tt.get(k), Some(u));
    }

    #[proptest]
    fn set_stores_transposition_if_none_exists(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        t: Transposition,
        k: Key,
    ) {
        tt.cache.store(tt.index_of(k), Bits::default());
        tt.set(k, t);
        assert_eq!(tt.get(k), Some(t));
    }
}
