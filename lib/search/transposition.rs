use crate::chess::{Move, Zobrist};
use crate::search::{Depth, HashSize, Score};
use crate::util::{Binary, Bits, Integer};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{mem::size_of, ops::RangeInclusive};

#[cfg(test)]
use crate::chess::Position;

#[cfg(test)]
use proptest::{collection::*, prelude::*};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(u8)]
enum TranspositionKind {
    Lower,
    Upper,
    Exact,
}

unsafe impl Integer for TranspositionKind {
    type Repr = u8;

    const MIN: Self::Repr = TranspositionKind::Lower as _;
    const MAX: Self::Repr = TranspositionKind::Exact as _;

    #[inline(always)]
    fn repr(&self) -> Self::Repr {
        *self as _
    }
}

impl Binary for TranspositionKind {
    type Bits = Bits<u8, 2>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        Bits::new(*self as _)
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        TranspositionKind::from_repr(bits.get())
    }
}

/// A partial search result.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Transposition {
    kind: TranspositionKind,
    depth: Depth,
    score: Score,
    best: Move,
}

impl Transposition {
    fn new(kind: TranspositionKind, depth: Depth, score: Score, best: Move) -> Self {
        Transposition {
            kind,
            depth,
            score,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, the depth searched, and best [`Move`].
    pub fn lower(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(TranspositionKind::Lower, depth, score, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, the depth searched, and best [`Move`].
    pub fn upper(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(TranspositionKind::Upper, depth, score, best)
    }

    /// Constructs a [`Transposition`] given the exact score, the depth searched, and best [`Move`].
    pub fn exact(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(TranspositionKind::Exact, depth, score, best)
    }

    /// Bounds for the exact score.
    pub fn bounds(&self) -> RangeInclusive<Score> {
        match self.kind {
            TranspositionKind::Lower => self.score..=Score::upper(),
            TranspositionKind::Upper => Score::lower()..=self.score,
            TranspositionKind::Exact => self.score..=self.score,
        }
    }

    /// Depth searched.
    pub fn depth(&self) -> Depth {
        self.depth
    }

    /// Partial score.
    pub fn score(&self) -> Score {
        self.score
    }

    /// Best [`Move`] at this depth.
    pub fn best(&self) -> Move {
        self.best
    }
}

impl Binary for Transposition {
    type Bits = Bits<u64, 37>;

    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.depth.encode());
        bits.push(self.kind.encode());
        bits.push(self.score.encode());
        bits.push(self.best.encode());
        bits
    }

    fn decode(mut bits: Self::Bits) -> Self {
        Transposition {
            best: Binary::decode(bits.pop()),
            score: Binary::decode(bits.pop()),
            kind: Binary::decode(bits.pop()),
            depth: Binary::decode(bits.pop()),
        }
    }
}

type Signature = Bits<u32, 27>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
struct SignedTransposition(Signature, <Transposition as Binary>::Bits);

impl Binary for SignedTransposition {
    type Bits = Bits<u64, 64>;

    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.1);
        bits.push(self.0);
        bits
    }

    fn decode(mut bits: Self::Bits) -> Self {
        SignedTransposition(bits.pop(), bits.pop())
    }
}

/// A cache for [`Transposition`]s.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct TranspositionTable {
    #[cfg_attr(test,
        strategy(hash_map(any::<Position>(), any::<Transposition>(), ..32).prop_map(|ts| {
            let mut cache: Box<[AtomicU64]> = (0..ts.len().next_power_of_two()).map(|_| AtomicU64::default()).collect();

            for (pos, t) in ts {
                let key = pos.zobrist();
                let idx = key.slice(..cache.len().trailing_zeros()).get() as usize;
                let sig = key.slice(cache.len().trailing_zeros()..).pop();
                *cache[idx].get_mut() = Some(SignedTransposition(sig, t.encode())).encode().get();
            }

            cache
        }))
    )]
    cache: Box<[AtomicU64]>,
}

impl TranspositionTable {
    const WIDTH: usize = size_of::<<Option<SignedTransposition> as Binary>::Bits>();

    /// Constructs a transposition table of at most `size` many bytes.
    pub fn new(size: HashSize) -> Self {
        let capacity = (1 + size.get() / 2).next_power_of_two() / Self::WIDTH;

        TranspositionTable {
            cache: (0..capacity).map(|_| AtomicU64::default()).collect(),
        }
    }

    /// The actual size of this table in bytes.
    pub fn size(&self) -> HashSize {
        HashSize::new(self.capacity() * Self::WIDTH)
    }

    /// The actual size of this table in number of entries.
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    fn signature_of(&self, key: Zobrist) -> Signature {
        key.slice(self.capacity().trailing_zeros()..).pop()
    }

    fn index_of(&self, key: Zobrist) -> usize {
        key.slice(..self.capacity().trailing_zeros()).get() as _
    }

    /// Loads the [`Transposition`] from the slot associated with `key`.
    pub fn get(&self, key: Zobrist) -> Option<Transposition> {
        if self.capacity() == 0 {
            return None;
        }

        let sig = self.signature_of(key);
        let bits = Bits::new(self.cache[self.index_of(key)].load(Ordering::Relaxed));
        match Binary::decode(bits) {
            Some(SignedTransposition(s, t)) if s == sig => Some(Binary::decode(t)),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater depth is chosen.
    pub fn set(&self, key: Zobrist, tpos: Transposition) {
        if self.capacity() > 0 {
            let sig = self.signature_of(key);
            let bits = Some(SignedTransposition(sig, tpos.encode())).encode();
            self.cache[self.index_of(key)].fetch_max(bits.get(), Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn lower_constructs_lower_bound_transposition(s: Score, d: Depth, m: Move) {
        assert_eq!(
            Transposition::lower(d, s, m),
            Transposition::new(TranspositionKind::Lower, d, s, m)
        );
    }

    #[proptest]
    fn upper_constructs_upper_bound_transposition(s: Score, d: Depth, m: Move) {
        assert_eq!(
            Transposition::upper(d, s, m),
            Transposition::new(TranspositionKind::Upper, d, s, m)
        );
    }

    #[proptest]
    fn exact_constructs_exact_transposition(s: Score, d: Depth, m: Move) {
        assert_eq!(
            Transposition::exact(d, s, m),
            Transposition::new(TranspositionKind::Exact, d, s, m)
        );
    }

    #[proptest]
    fn transposition_score_is_between_bounds(t: Transposition) {
        assert!(t.bounds().contains(&t.score()));
    }

    #[proptest]
    fn decoding_encoded_transposition_kind_is_an_identity(t: TranspositionKind) {
        assert_eq!(TranspositionKind::decode(t.encode()), t);
    }

    #[proptest]
    fn decoding_encoded_transposition_is_an_identity(t: Transposition) {
        assert_eq!(Transposition::decode(t.encode()), t);
    }

    #[proptest]
    fn decoding_encoded_signed_transposition_is_an_identity(t: SignedTransposition) {
        assert_eq!(SignedTransposition::decode(t.encode()), t);
    }

    #[proptest]
    fn table_input_size_is_an_upper_limit(s: HashSize) {
        assert!(TranspositionTable::new(s).size() <= s);
    }

    #[proptest]
    fn table_size_is_exact_if_input_is_power_of_two(
        #[strategy(TranspositionTable::WIDTH.trailing_zeros()..=HashSize::max().get().trailing_zeros())]
        bits: u32,
    ) {
        let s = HashSize::new(1 << bits);
        assert_eq!(TranspositionTable::new(s).size(), s);
    }

    #[proptest]
    fn table_capacity_equals_the_size_in_bytes(tt: TranspositionTable) {
        assert_eq!(tt.size(), tt.cache.len() * TranspositionTable::WIDTH);
    }

    #[proptest]
    fn get_does_nothing_if_capacity_is_zero(k: Zobrist) {
        assert_eq!(TranspositionTable::new(HashSize::new(0)).get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_transposition_does_not_exist(mut tt: TranspositionTable, k: Zobrist) {
        *tt.cache[tt.index_of(k)].get_mut() = 0;
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_signature_does_not_match(
        mut tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(!tt.signature_of(k), t.encode()));
        *tt.cache[tt.index_of(k)].get_mut() = st.encode().get();
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(
        mut tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(tt.signature_of(k), t.encode()));
        *tt.cache[tt.index_of(k)].get_mut() = st.encode().get();
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_does_nothing_if_capacity_is_zero(k: Zobrist, t: Transposition) {
        TranspositionTable::new(HashSize::new(0)).set(k, t);
    }

    #[proptest]
    fn set_keeps_transposition_with_greater_depth(
        #[by_ref] mut tt: TranspositionTable,
        t: Transposition,
        #[filter(#t.depth() != #u.depth())] u: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(tt.signature_of(k), t.encode()));
        *tt.cache[tt.index_of(k)].get_mut() = st.encode().get();
        tt.set(k, u);

        if t.depth() > u.depth() {
            assert_eq!(tt.get(k), Some(t));
        } else {
            assert_eq!(tt.get(k), Some(u));
        }
    }

    #[proptest]
    fn set_ignores_the_signature_mismatch(
        #[by_ref] mut tt: TranspositionTable,
        t: Transposition,
        #[filter(#u.depth() > #t.depth())] u: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(!tt.signature_of(k), t.encode()));
        *tt.cache[tt.index_of(k)].get_mut() = st.encode().get();
        tt.set(k, u);
        assert_eq!(tt.get(k), Some(u));
    }

    #[proptest]
    fn set_stores_transposition_if_none_exists(
        mut tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        *tt.cache[tt.index_of(k)].get_mut() = 0;
        tt.set(k, t);
        assert_eq!(tt.get(k), Some(t));
    }
}
