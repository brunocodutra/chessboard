use crate::chess::{Move, Zobrist};
use crate::search::{Depth, Score};
use crate::util::{Assume, Binary, Bits, Cache};
use derive_more::{Display, Error};
use std::{cmp::Ordering, mem::size_of, ops::RangeInclusive};

#[cfg(test)]
use crate::chess::Position;

#[cfg(test)]
use proptest::{collection::*, prelude::*};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
enum Kind {
    Lower,
    Upper,
    Exact,
}

/// A partial search result.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Transposition {
    kind: Kind,
    depth: Depth,
    score: Score,
    best: Move,
}

impl PartialOrd for Transposition {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Transposition {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.depth, self.kind).cmp(&(other.depth, other.kind))
    }
}

impl Transposition {
    fn new(kind: Kind, depth: Depth, score: Score, best: Move) -> Self {
        Transposition {
            kind,
            depth,
            score,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, the depth searched, and best [`Move`].
    pub fn lower(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(Kind::Lower, depth, score, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, the depth searched, and best [`Move`].
    pub fn upper(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(Kind::Upper, depth, score, best)
    }

    /// Constructs a [`Transposition`] given the exact score, the depth searched, and best [`Move`].
    pub fn exact(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(Kind::Exact, depth, score, best)
    }

    /// Bounds for the exact score.
    pub fn bounds(&self) -> RangeInclusive<Score> {
        match self.kind {
            Kind::Lower => self.score..=Score::UPPER,
            Kind::Upper => Score::LOWER..=self.score,
            Kind::Exact => self.score..=self.score,
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

type Signature = Bits<u32, 24>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
struct SignedTransposition(Transposition, Signature);

/// The reason why decoding [`Transposition`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "not a valid transposition")]
pub struct DecodeTranspositionError;

impl Binary for SignedTransposition {
    type Bits = Bits<u64, 64>;
    type Error = DecodeTranspositionError;

    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.1.encode());
        bits.push(self.0.best.encode());
        bits.push(self.0.score.encode());
        bits.push(self.0.depth.encode());
        bits.push(Bits::<u8, 2>::new(self.0.kind as _));
        bits
    }

    fn decode(mut bits: Self::Bits) -> Result<Self, Self::Error> {
        Ok(SignedTransposition(
            Transposition {
                kind: [Kind::Lower, Kind::Upper, Kind::Exact]
                    .into_iter()
                    .nth(bits.pop::<_, 2>().get())
                    .ok_or(DecodeTranspositionError)?,
                depth: Depth::decode(bits.pop()).map_err(|_| DecodeTranspositionError)?,
                score: Score::decode(bits.pop()).map_err(|_| DecodeTranspositionError)?,
                best: Move::decode(bits.pop()).map_err(|_| DecodeTranspositionError)?,
            },
            Signature::decode(bits.pop()).map_err(|_| DecodeTranspositionError)?,
        ))
    }
}

/// A cache for [`Transposition`]s.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct TranspositionTable {
    #[cfg_attr(test,
        strategy((..=32usize, hash_map(any::<Position>(), any::<Transposition>(), ..=32)).prop_map(|(cap, ts)| {
            let cache = Cache::new(cap.next_power_of_two());

            for (pos, t) in ts {
                let key = pos.zobrist();
                let idx = key.slice(..cache.len().trailing_zeros()).get() as _;
                let sig = key.slice(cache.len().trailing_zeros()..).pop();
                cache.store(idx, Some(SignedTransposition(t, sig)).encode());
            }

            cache
        }))
    )]
    cache: Cache<<Option<SignedTransposition> as Binary>::Bits>,
}

impl TranspositionTable {
    /// Constructs a transposition table of at most `size` many bytes.
    ///
    /// The `size` specifies an upper bound, as long as the table is not empty.
    pub fn new(cap: usize) -> Self {
        let entry_size = size_of::<<Option<SignedTransposition> as Binary>::Bits>();

        TranspositionTable {
            cache: Cache::new(((cap + entry_size) / entry_size).next_power_of_two() / 2),
        }
    }

    /// The actual size of this table in bytes.
    pub fn size(&self) -> usize {
        self.capacity() * size_of::<<Option<SignedTransposition> as Binary>::Bits>()
    }

    /// The actual size of this table in number of entries.
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    /// Clears the table.
    pub fn clear(&mut self) {
        self.cache.clear()
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
        let bits = self.cache.load(self.index_of(key));
        match Binary::decode(bits).assume() {
            Some(SignedTransposition(t, s)) if s == sig => Some(t),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater depth is chosen.
    pub fn set(&self, key: Zobrist, transposition: Transposition) {
        if self.capacity() > 0 {
            let sig = self.signature_of(key);
            let bits = Some(SignedTransposition(transposition, sig)).encode();
            self.cache
                .update(self.index_of(key), |r| match Binary::decode(r).assume() {
                    Some(SignedTransposition(t, _)) if t > transposition => None,
                    _ => Some(bits),
                })
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
            Transposition::new(Kind::Lower, d, s, m)
        );
    }

    #[proptest]
    fn upper_constructs_upper_bound_transposition(s: Score, d: Depth, m: Move) {
        assert_eq!(
            Transposition::upper(d, s, m),
            Transposition::new(Kind::Upper, d, s, m)
        );
    }

    #[proptest]
    fn exact_constructs_exact_transposition(s: Score, d: Depth, m: Move) {
        assert_eq!(
            Transposition::exact(d, s, m),
            Transposition::new(Kind::Exact, d, s, m)
        );
    }

    #[proptest]
    fn transposition_score_is_between_bounds(t: Transposition) {
        assert!(t.bounds().contains(&t.score()));
    }

    #[proptest]
    fn transposition_with_larger_depth_is_larger(
        t: Transposition,
        #[filter(#t.depth() != #u.depth())] u: Transposition,
    ) {
        assert_eq!(t < u, t.depth() < u.depth());
    }

    #[proptest]
    fn transpositions_with_same_depth_are_compared_by_kind(
        t: Transposition,
        #[filter(#t.depth() == #u.depth())] u: Transposition,
    ) {
        assert_eq!(t < u, t.kind < u.kind);
    }

    #[proptest]
    fn decoding_encoded_transposition_is_an_identity(t: SignedTransposition) {
        assert_eq!(Binary::decode(t.encode()), Ok(t));
    }

    #[proptest]
    fn table_input_size_is_an_upper_limit(#[strategy(..=1024usize)] s: usize) {
        assert!(TranspositionTable::new(s).size() <= s);
    }

    #[proptest]
    fn table_size_is_exact_if_input_is_power_of_two(
        #[strategy(size_of::<<Option<SignedTransposition> as Binary>::Bits>()..=1024)] s: usize,
    ) {
        assert_eq!(
            TranspositionTable::new(s.next_power_of_two()).size(),
            s.next_power_of_two()
        );
    }

    #[proptest]
    fn table_capacity_equals_the_size_in_bytes(tt: TranspositionTable) {
        assert_eq!(
            tt.size(),
            tt.cache.len() * size_of::<<Option<SignedTransposition> as Binary>::Bits>()
        );
    }

    #[proptest]
    fn get_does_nothing_if_capacity_is_zero(k: Zobrist) {
        assert_eq!(TranspositionTable::new(0).get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_transposition_does_not_exist(tt: TranspositionTable, k: Zobrist) {
        tt.cache.store(tt.index_of(k), Bits::default());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_signature_does_not_match(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(t, !tt.signature_of(k)));
        tt.cache.store(tt.index_of(k), st.encode());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(t, tt.signature_of(k)));
        tt.cache.store(tt.index_of(k), st.encode());
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_does_nothing_if_capacity_is_zero(k: Zobrist, t: Transposition) {
        TranspositionTable::new(0).set(k, t);
    }

    #[proptest]
    fn set_keeps_greater_transposition(
        tt: TranspositionTable,
        t: Transposition,
        u: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(t, tt.signature_of(k)));
        tt.cache.store(tt.index_of(k), st.encode());
        tt.set(k, u);

        if t > u {
            assert_eq!(tt.get(k), Some(t));
        } else {
            assert_eq!(tt.get(k), Some(u));
        }
    }

    #[proptest]
    fn set_ignores_the_signature_mismatch(
        #[by_ref] tt: TranspositionTable,
        t: Transposition,
        #[filter(#u.depth() > #t.depth())] u: Transposition,
        k: Zobrist,
    ) {
        let st = Some(SignedTransposition(t, !tt.signature_of(k)));
        tt.cache.store(tt.index_of(k), st.encode());
        tt.set(k, u);
        assert_eq!(tt.get(k), Some(u));
    }

    #[proptest]
    fn set_stores_transposition_if_none_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        tt.cache.store(tt.index_of(k), Bits::default());
        tt.set(k, t);
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn clear_resets_cache(mut tt: TranspositionTable, k: Zobrist) {
        tt.clear();
        assert_eq!(tt.get(k), None);
    }
}
