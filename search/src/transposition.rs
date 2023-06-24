use chess::{Move, Position, Zobrist};
use derive_more::{Display, Error};
use proptest::{collection::*, prelude::*};
use std::{cmp::Ordering, mem::size_of, ops::RangeInclusive};
use test_strategy::Arbitrary;
use util::{Binary, Bits, Cache, Depth, Score};

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
    depth: Depth,
    score: Score,
    best: Move,
}

impl PartialOrd for Transposition {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.depth, self.kind).partial_cmp(&(other.depth, other.kind))
    }
}

impl Ord for Transposition {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        (self.depth, self.kind).cmp(&(other.depth, other.kind))
    }
}

impl Transposition {
    #[inline]
    fn new(kind: Kind, depth: Depth, score: Score, best: Move) -> Self {
        Transposition {
            kind,
            depth,
            score,
            best,
        }
    }

    /// Constructs a [`Transposition`] given a lower bound for the score, the depth searched, and best [`Move`].
    #[inline]
    pub fn lower(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(Kind::Lower, depth, score, best)
    }

    /// Constructs a [`Transposition`] given an upper bound for the score, the depth searched, and best [`Move`].
    #[inline]
    pub fn upper(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(Kind::Upper, depth, score, best)
    }

    /// Constructs a [`Transposition`] given the exact score, the depth searched, and best [`Move`].
    #[inline]
    pub fn exact(depth: Depth, score: Score, best: Move) -> Self {
        Transposition::new(Kind::Exact, depth, score, best)
    }

    /// Bounds for the exact score.
    #[inline]
    pub fn bounds(&self) -> RangeInclusive<Score> {
        match self.kind {
            Kind::Lower => self.score..=Score::upper(),
            Kind::Upper => Score::lower()..=self.score,
            Kind::Exact => self.score..=self.score,
        }
    }

    /// Depth searched.
    #[inline]
    pub fn depth(&self) -> Depth {
        self.depth
    }

    /// Partial score.
    #[inline]
    pub fn score(&self) -> Score {
        self.score
    }

    /// Best [`Move`] at this depth.
    #[inline]
    pub fn best(&self) -> Move {
        self.best
    }
}

type Signature = Bits<u32, 29>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
struct SignedTransposition(Transposition, Signature);

/// The reason why decoding [`Transposition`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid transposition")]
pub struct DecodeTranspositionError;

impl Binary for SignedTransposition {
    type Bits = Bits<u64, 64>;
    type Error = DecodeTranspositionError;

    #[inline]
    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.1.encode());
        bits.push(self.0.best.encode());
        bits.push(self.0.score.encode());
        bits.push(self.0.depth.encode());
        bits.push(Bits::<u8, 2>::new(self.0.kind as _));
        bits
    }

    #[inline]
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

/// An iterator over a sequence of [`Transposition`]s in a transposition [`Table`].
#[derive(Debug, Clone)]
pub struct TranspositionIter<'a> {
    tt: &'a TranspositionTable,
    pos: Position,
    depth: u8,
}

impl<'a> TranspositionIter<'a> {
    #[inline]
    pub fn new(tt: &'a TranspositionTable, pos: Position) -> Self {
        TranspositionIter {
            tt,
            pos,
            depth: u8::MAX,
        }
    }
}

impl<'a> Iterator for TranspositionIter<'a> {
    type Item = Transposition;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.pos.zobrist();
        let t = self.tt.get(key).filter(|t| t.depth() <= self.depth)?;
        self.depth = t.depth().get().checked_sub(1)?;
        self.pos.play(t.best()).ok()?;
        Some(t)
    }
}

/// A cache for [`Transposition`]s.
#[derive(Debug, Arbitrary)]
pub struct TranspositionTable {
    #[strategy((1usize..=32, hash_map(any::<Position>(), any::<Transposition>(), 0..=32)).prop_map(|(cap, ts)| {
        let cache = Cache::new(cap.next_power_of_two());

        for (pos, t) in ts {
            let key = pos.zobrist();
            let idx = key.slice(..cache.len().trailing_zeros()).get() as _;
            let sig = key.slice(cache.len().trailing_zeros()..).pop();
            cache.store(idx, Some(SignedTransposition(t, sig)).encode());
        }

        cache
    }))]
    cache: Cache<<Option<SignedTransposition> as Binary>::Bits>,
}

impl TranspositionTable {
    /// Constructs a transposition [`Table`] of at most `size` many bytes.
    ///
    /// The `size` specifies an upper bound, as long as the table is not empty.
    pub fn new(size: usize) -> Self {
        let entry_size = size_of::<<Option<SignedTransposition> as Binary>::Bits>();
        let cache_size = (size / entry_size + 1).next_power_of_two() / 2;

        TranspositionTable {
            cache: Cache::new(cache_size.max(1)),
        }
    }

    /// The actual size of this [`Table`] in bytes.
    #[inline]
    pub fn size(&self) -> usize {
        self.capacity() * size_of::<<Option<SignedTransposition> as Binary>::Bits>()
    }

    /// The actual size of this [`Table`] in number of entries.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    /// Clears the table.
    #[inline]
    pub fn clear(&mut self) {
        self.cache.clear()
    }

    #[inline]
    fn signature_of(&self, key: Zobrist) -> Signature {
        key.slice(self.capacity().trailing_zeros()..).pop()
    }

    #[inline]
    fn index_of(&self, key: Zobrist) -> usize {
        key.slice(..self.capacity().trailing_zeros()).get() as _
    }

    /// Loads the [`Transposition`] from the slot associated with `key`.
    #[inline]
    pub fn get(&self, key: Zobrist) -> Option<Transposition> {
        let sig = self.signature_of(key);
        let bits = self.cache.load(self.index_of(key));
        match Binary::decode(bits).expect("expected valid encoding") {
            Some(SignedTransposition(t, s)) if s == sig => Some(t),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater depth is chosen.
    #[inline]
    pub fn set(&self, key: Zobrist, transposition: Transposition) {
        let sig = self.signature_of(key);
        let bits = Some(SignedTransposition(transposition, sig)).encode();
        self.cache.update(self.index_of(key), |r| {
            match Binary::decode(r).expect("expected valid encoding") {
                Some(SignedTransposition(t, _)) if t > transposition => None,
                _ => Some(bits),
            }
        })
    }

    /// Clears the [`Transposition`] from the slot associated with `key`.
    #[inline]
    pub fn unset(&self, key: Zobrist) {
        self.cache.store(self.index_of(key), Bits::default())
    }

    /// An iterator for a sequence of [`Transposition`]s from a starting [`Position`].
    #[inline]
    pub fn iter(&self, pos: &Position) -> impl Iterator<Item = Transposition> + '_ {
        TranspositionIter::new(self, pos.clone())
    }

    /// An iterator for a sequence of moves from a starting [`Position`].
    #[inline]
    pub fn line(&self, pos: &Position) -> impl Iterator<Item = Move> + '_ {
        self.iter(pos).map(|t| t.best())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess::MoveKind;
    use proptest::sample::Selector;
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
    fn decoding_encoded_transposition_is_an_identity(t: SignedTransposition) {
        assert_eq!(Binary::decode(t.encode()), Ok(t));
    }

    #[proptest]
    fn iterates_over_moves_searched_with_strictly_decreasing_depth(
        #[by_ref]
        #[filter(#tt.capacity() > 1)]
        tt: TranspositionTable,
        #[filter(#pos.moves(MoveKind::ANY).len() > 0)] pos: Position,
        #[filter(#d > Depth::new(0))] d: Depth,
        s: Score,
        selector: Selector,
    ) {
        let (m, next) = selector.select(pos.moves(MoveKind::ANY));
        prop_assume!(next.moves(MoveKind::ANY).len() > 0);

        let (n, _) = selector.select(next.moves(MoveKind::ANY));

        let t = Transposition::lower(d, s, m);
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);

        let u = Transposition::lower(d, -s, n);
        tt.unset(next.zobrist());
        tt.set(next.zobrist(), u);

        prop_assume!(tt.get(pos.zobrist()) == Some(t));
        prop_assume!(tt.get(next.zobrist()) == Some(u));

        let mut it = TranspositionIter::new(&tt, pos);
        assert_eq!(it.next(), Some(t));
        assert_eq!(it.next(), None);
    }

    #[proptest]
    fn iterates_over_legal_moves_only(
        #[by_ref] tt: TranspositionTable,
        pos: Position,
        #[filter(#pos.clone().play(#t.best()).is_err())] t: Transposition,
    ) {
        tt.unset(pos.zobrist());
        tt.set(pos.zobrist(), t);
        assert_eq!(TranspositionIter::new(&tt, pos).next(), None);
    }

    #[proptest]
    fn is_fused(tt: TranspositionTable, pos: Position) {
        let mut it = TranspositionIter::new(&tt, pos);

        while it.next().is_some() {}

        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None);
    }

    #[proptest]
    fn size_returns_table_capacity_in_bytes(tt: TranspositionTable) {
        assert_eq!(
            tt.size(),
            tt.cache.len() * size_of::<<Option<SignedTransposition> as Binary>::Bits>()
        );
    }

    #[proptest]
    fn input_size_is_an_upper_limit(
        #[strategy(size_of::<<Option<SignedTransposition> as Binary>::Bits>()..=1024)] s: usize,
    ) {
        assert!(TranspositionTable::new(s).size() <= s);
    }

    #[proptest]
    fn size_is_exact_if_input_is_power_of_two(
        #[strategy(size_of::<<Option<SignedTransposition> as Binary>::Bits>()..=1024)] s: usize,
    ) {
        assert_eq!(
            TranspositionTable::new(s.next_power_of_two()).size(),
            s.next_power_of_two()
        );
    }

    #[proptest]
    fn capacity_returns_cache_len(tt: TranspositionTable) {
        assert_eq!(tt.capacity(), tt.cache.len());
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
    fn unset_erases_transposition(tt: TranspositionTable, k: Zobrist) {
        tt.unset(k);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn clear_resets_cache(mut tt: TranspositionTable, k: Zobrist) {
        tt.clear();
        assert_eq!(tt.get(k), None);
    }
}
