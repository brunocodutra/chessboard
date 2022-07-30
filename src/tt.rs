use crate::{Binary, Bits, Cache, Move, Register};
use bitvec::{field::BitField, mem::BitRegister, store::BitStore};
use derive_more::{Display, Error};
use std::{cmp::Ordering, fmt::Debug, ops::Deref};

#[cfg(test)]
use proptest::prelude::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
enum TranspositionKind {
    Lower,
    Upper,
    Exact,
}

/// A partial search result.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Transposition {
    kind: TranspositionKind,
    score: i16,
    #[cfg_attr(test, strategy(0i8..))]
    draft: i8,
    best: Option<Move>,
}

impl Transposition {
    fn new(kind: TranspositionKind, score: i16, draft: i8, best: Option<Move>) -> Self {
        Transposition {
            kind,
            score,
            draft,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, remaining draft, and best [`Move`].
    pub fn lower(score: i16, draft: i8, best: Option<Move>) -> Self {
        Transposition::new(TranspositionKind::Lower, score, draft, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, remaining draft, and best [`Move`].
    pub fn upper(score: i16, draft: i8, best: Option<Move>) -> Self {
        Transposition::new(TranspositionKind::Upper, score, draft, best)
    }

    /// Constructs a [`Transposition`] given the exact score, remaining draft, and best [`Move`].
    pub fn exact(score: i16, draft: i8, best: Option<Move>) -> Self {
        Transposition::new(TranspositionKind::Exact, score, draft, best)
    }

    /// Bounds for the exact score.
    pub fn bounds(&self) -> (i16, i16) {
        match self.kind {
            TranspositionKind::Lower => (self.score, i16::MAX),
            TranspositionKind::Upper => (i16::MIN, self.score),
            TranspositionKind::Exact => (self.score, self.score),
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
    pub fn best(&self) -> Option<Move> {
        self.best
    }
}

impl PartialOrd for Transposition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.draft, self.kind).partial_cmp(&(other.draft, other.kind))
    }
}

impl Ord for Transposition {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.draft, self.kind).cmp(&(other.draft, other.kind))
    }
}

type Key = Bits<u64, 64>;
type Signature = Bits<u32, 24>;
type OptionalSignedTransposition = Option<(Transposition, Signature)>;
type OptionalSignedTranspositionRegister = <OptionalSignedTransposition as Binary>::Register;

/// The reason why decoding [`Transposition`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Error)]
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
                let (kind, rest) = register.split_at_mut(2);
                let (score, rest) = rest.split_at_mut(16);
                let (draft, rest) = rest.split_at_mut(7);
                let (best, rest) = rest.split_at_mut(<Move as Binary>::Register::WIDTH);

                kind.store(t.kind as u8 + 1);
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
            let (kind, rest) = register.split_at(2);
            let (score, rest) = rest.split_at(16);
            let (draft, rest) = rest.split_at(7);
            let (best, rest) = rest.split_at(<Move as Binary>::Register::WIDTH);

            use TranspositionKind::*;
            Ok(Some((
                Transposition {
                    kind: [Lower, Upper, Exact]
                        .into_iter()
                        .nth((kind.load::<usize>() + 2) % 3)
                        .ok_or(DecodeTranspositionError(register))?,
                    score: score.load(),
                    draft: draft.load::<u8>() as i8,
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
        if !self.is_empty() {
            OptionalSignedTransposition::decode(self.cache.load(self.index_of(key)))
                .expect("expected valid encoding")
                .filter(|(_, sig)| sig.deref() == key[(Key::WIDTH - Signature::WIDTH)..])
                .map(|(t, _)| t)
        } else {
            None
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Ordering::Greater`] [`Transposition`] is chosen.
    pub fn set(&self, key: Key, transposition: Transposition) {
        if !self.is_empty() {
            let sig = key[(Key::WIDTH - Signature::WIDTH)..].into();
            self.cache.update(self.index_of(key), |r| {
                match Binary::decode(r).expect("expected valid encoding") {
                    Some((t, _)) if t > transposition => None,
                    _ => Some(Some((transposition, sig)).encode()),
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
    fn transposition_score_is_between_bounds(t: Transposition) {
        let (lower, upper) = t.bounds();
        assert!(lower <= t.score());
        assert!(t.score() <= upper);
    }

    #[proptest]
    fn transposition_with_larger_draft_is_larger(
        a: Transposition,
        #[filter(#a.draft != #b.draft)] b: Transposition,
    ) {
        assert_eq!(a < b, a.draft < b.draft);
    }

    #[proptest]
    fn transposition_with_same_draft_is_compared_by_kind(
        a: Transposition,
        b: Transposition,
        d: i8,
    ) {
        assert_eq!(
            Transposition { draft: d, ..a } < Transposition { draft: d, ..b },
            a.kind < b.kind
        );
    }

    #[proptest]
    fn decoding_encoded_transposition_is_an_identity(r: OptionalSignedTransposition) {
        assert_eq!(Binary::decode(r.encode()), Ok(r));
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
        let sig = (!k.load::<u64>()).view_bits()[40..].into();
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
        tt.cache
            .store(tt.index_of(k), Some((t, k[40..].into())).encode());
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_does_nothing_if_table_is_empty(t: Transposition, k: Key) {
        let tt = TranspositionTable::new(0);
        tt.set(k, t);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn set_keeps_transposition_with_higher_rank_or_newer(
        #[by_ref]
        #[filter(!#tt.is_empty())]
        tt: TranspositionTable,
        a: Transposition,
        b: Transposition,
        k: Key,
    ) {
        tt.cache
            .store(tt.index_of(k), Some((a, k[40..].into())).encode());
        tt.set(k, b);
        assert_eq!(tt.get(k), if a > b { Some(a) } else { Some(b) });
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
