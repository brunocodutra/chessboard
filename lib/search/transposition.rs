use crate::chess::{Move, Zobrist};
use crate::search::{Depth, HashSize, Ply, Pv, Score};
use crate::util::{Assume, Binary, Bits, Integer};
use derive_more::Debug;
use std::mem::size_of;
use std::ops::{Index, RangeInclusive};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

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
}

impl Binary for TranspositionKind {
    type Bits = Bits<u8, 2>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        self.convert().assume()
    }

    #[inline(always)]
    fn decode(bits: Self::Bits) -> Self {
        bits.convert().assume()
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
    #[inline(always)]
    fn new(kind: TranspositionKind, depth: Depth, score: Score, best: Move) -> Self {
        Transposition {
            kind,
            depth,
            score,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, the depth searched, and best [`Move`].
    #[inline(always)]
    pub fn lower(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(TranspositionKind::Lower, depth, score, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, the depth searched, and best [`Move`].
    #[inline(always)]
    pub fn upper(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(TranspositionKind::Upper, depth, score, best)
    }

    /// Constructs a [`Transposition`] given the exact score, the depth searched, and best [`Move`].
    #[inline(always)]
    pub fn exact(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(TranspositionKind::Exact, depth, score, best)
    }

    /// Bounds for the exact score.
    #[inline(always)]
    pub fn bounds(&self) -> RangeInclusive<Score> {
        match self.kind {
            TranspositionKind::Lower => self.score..=Score::upper(),
            TranspositionKind::Upper => Score::lower()..=self.score,
            TranspositionKind::Exact => self.score..=self.score,
        }
    }

    /// Depth searched.
    #[inline(always)]
    pub fn depth(&self) -> Depth {
        self.depth
    }

    /// Partial score.
    #[inline(always)]
    pub fn score(&self) -> Score {
        self.score
    }

    /// Principal variation normalized to [`Ply`].
    #[inline(always)]
    pub fn transpose(&self, ply: Ply) -> Pv<1> {
        Pv::new(self.score().normalize(ply), [self.best])
    }
}

impl Binary for Transposition {
    type Bits = Bits<u64, 37>;

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.depth.encode());
        bits.push(self.kind.encode());
        bits.push(self.score.encode());
        bits.push(self.best.encode());
        bits
    }

    #[inline(always)]
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

    #[inline(always)]
    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.1);
        bits.push(self.0);
        bits
    }

    #[inline(always)]
    fn decode(mut bits: Self::Bits) -> Self {
        SignedTransposition(bits.pop(), bits.pop())
    }
}

/// A cache for [`Transposition`]s.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("TranspositionTable({})", self.capacity())]
pub struct TranspositionTable {
    #[cfg_attr(test,
        strategy(hash_map(any::<Position>(), any::<Transposition>(), ..32).prop_map(|ts| {
            let mut cache: Box<[AtomicU64]> = (0..ts.len().next_power_of_two()).map(|_| AtomicU64::default()).collect();

            for (pos, t) in ts {
                let key = pos.zobrist();
                let idx = key.slice(..cache.len().trailing_zeros()).cast::<usize>();
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
    #[inline(always)]
    pub fn new(size: HashSize) -> Self {
        let capacity = (1 + size.get() / 2).next_power_of_two() / Self::WIDTH;

        TranspositionTable {
            cache: (0..capacity).map(|_| AtomicU64::default()).collect(),
        }
    }

    /// The actual size of this table in bytes.
    #[inline(always)]
    pub fn size(&self) -> HashSize {
        HashSize::new(self.capacity() * Self::WIDTH)
    }

    /// The actual size of this table in number of entries.
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    /// Instructs the CPU to load the slot associated with `key` onto the cache.
    #[inline(always)]
    pub fn prefetch(&self, key: Zobrist) {
        if self.capacity() > 0 {
            #[cfg(target_arch = "x86_64")]
            unsafe {
                use std::arch::x86_64::{_mm_prefetch, _MM_HINT_ET0};
                _mm_prefetch(self.cache[key].as_ptr() as _, _MM_HINT_ET0);
            }
        }
    }

    /// Loads the [`Transposition`] from the slot associated with `key`.
    #[inline(always)]
    pub fn get(&self, key: Zobrist) -> Option<Transposition> {
        if self.capacity() == 0 {
            return None;
        }

        let sig = self.sign(key);
        let bits = Bits::new(self.cache[key].load(Relaxed));
        match Binary::decode(bits) {
            Some(SignedTransposition(s, t)) if s == sig => Some(Binary::decode(t)),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater depth is chosen.
    #[inline(always)]
    pub fn set(&self, key: Zobrist, tpos: Transposition) {
        if self.capacity() > 0 {
            let sig = self.sign(key);
            let bits = Some(SignedTransposition(sig, tpos.encode())).encode();
            self.cache[key].store(bits.get(), Relaxed);
        }
    }

    /// Returns the [`Signature`] associated with `key`.
    #[inline(always)]
    pub fn sign(&self, key: Zobrist) -> Signature {
        key.slice(self.capacity().trailing_zeros()..).pop()
    }
}

impl Index<Zobrist> for [AtomicU64] {
    type Output = AtomicU64;

    #[inline(always)]
    fn index(&self, key: Zobrist) -> &Self::Output {
        let idx: usize = key.slice(..self.len().trailing_zeros()).cast();
        self.get(idx).assume()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;
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
        #[strategy(TranspositionTable::WIDTH.trailing_zeros()..=HashSize::MAX.trailing_zeros())]
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
    fn get_returns_none_if_transposition_does_not_exist(tt: TranspositionTable, k: Zobrist) {
        tt.cache[k].store(0, Relaxed);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_signature_does_not_match(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(!tt.sign(k), t.encode()));
        tt.cache[k].store(st.encode().get(), Relaxed);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(tt.sign(k), t.encode()));
        tt.cache[k].store(st.encode().get(), Relaxed);
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_does_nothing_if_capacity_is_zero(k: Zobrist, t: Transposition) {
        TranspositionTable::new(HashSize::new(0)).set(k, t);
    }

    #[proptest]
    fn set_replaces_transposition_if_one_exists(
        #[by_ref] tt: TranspositionTable,
        s: Signature,
        t: Transposition,
        u: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(s, t.encode()));
        tt.cache[k].store(st.encode().get(), Relaxed);
        tt.set(k, u);
        assert_eq!(tt.get(k), Some(u));
    }

    #[proptest]
    fn set_stores_transposition_if_none_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        tt.cache[k].store(None::<SignedTransposition>.encode().get(), Relaxed);
        tt.set(k, t);
        assert_eq!(tt.get(k), Some(t));
    }
}
