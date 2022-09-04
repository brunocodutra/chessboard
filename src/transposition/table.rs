use super::{OptionalSignedTranspositionRegister, Signature, Transposition, TranspositionIterator};
use crate::chess::{Position, Zobrist};
use crate::util::{Binary, Cache, Register};
use bitvec::field::BitField;

#[cfg(test)]
use proptest::{collection::*, prelude::*};

/// A cache for [`Transposition`]s.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct TranspositionTable {
    #[cfg_attr(test, strategy(
        (1usize..=128, hash_map(any::<Position>(), any::<Transposition>(), 0..=32)).prop_map(|(cap, ts)| {
            let cache = Cache::new(cap);

            for (pos, t) in ts {
                let key = pos.zobrist();
                let idx = key.load::<usize>() & (cache.len() - 1);
                let sig = key[(Zobrist::WIDTH - Signature::WIDTH)..].into();
                cache.store(idx, Some((t, sig)).encode());
            }

            cache
        })
        .no_shrink()
    ))]
    cache: Cache<OptionalSignedTranspositionRegister>,
}

impl TranspositionTable {
    /// Constructs a [`TranspositionTable`] of at most `size` many bytes.
    ///
    /// The `size` specifies an upper bound, as long as the table is not empty.
    pub fn new(size: usize) -> Self {
        let entry_size = OptionalSignedTranspositionRegister::SIZE;
        let cache_size = (size / entry_size + 1).next_power_of_two() / 2;

        TranspositionTable {
            cache: Cache::new(cache_size.max(1)),
        }
    }

    /// The actual size of this [`TranspositionTable`] in bytes.
    pub fn size(&self) -> usize {
        self.capacity() * OptionalSignedTranspositionRegister::SIZE
    }

    /// The actual size of this [`TranspositionTable`] in number of entries.
    pub fn capacity(&self) -> usize {
        self.cache.len()
    }

    /// Clears the table.
    pub fn clear(&mut self) {
        self.cache.clear()
    }

    fn signature_of(&self, key: Zobrist) -> Signature {
        key[(Zobrist::WIDTH - Signature::WIDTH)..].into()
    }

    fn index_of(&self, key: Zobrist) -> usize {
        key.load::<usize>() & (self.capacity() - 1)
    }

    /// Loads the [`Transposition`] from the slot associated with `key`.
    pub fn get(&self, key: Zobrist) -> Option<Transposition> {
        let sig = self.signature_of(key);
        let register = self.cache.load(self.index_of(key));
        match Binary::decode(register).expect("expected valid encoding") {
            Some((t, s)) if s == sig => Some(t),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater draft is chosen.
    pub fn set(&self, key: Zobrist, transposition: Transposition) {
        let sig = self.signature_of(key);
        let register = Some((transposition, sig)).encode();
        self.cache.update(self.index_of(key), |r| {
            match Binary::decode(r).expect("expected valid encoding") {
                Some((t, _)) if t > transposition => None,
                _ => Some(register),
            }
        })
    }

    /// Clears the [`Transposition`] from the slot associated with `key`.
    pub fn unset(&self, key: Zobrist) {
        self.cache.store(self.index_of(key), None.encode())
    }

    /// An iterator for the principal variation from a starting [`Position`].
    pub fn iter(&self, pos: &Position) -> TranspositionIterator<'_> {
        TranspositionIterator::new(self, pos.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Bits;
    use bitvec::view::BitView;
    use test_strategy::proptest;

    #[proptest]
    fn size_returns_table_capacity_in_bytes(tt: TranspositionTable) {
        assert_eq!(
            tt.size(),
            tt.cache.len() * OptionalSignedTranspositionRegister::SIZE
        );
    }

    #[proptest]
    fn input_size_is_an_upper_limit(
        #[strategy(OptionalSignedTranspositionRegister::SIZE..=128)] s: usize,
    ) {
        assert!(TranspositionTable::new(s).size() <= s);
    }

    #[proptest]
    fn size_is_exact_if_input_is_power_of_two(
        #[strategy(OptionalSignedTranspositionRegister::SIZE..=128)] s: usize,
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
        let sig = tt.signature_of((!k.load::<u64>()).view_bits().into());
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(
        tt: TranspositionTable,
        t: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_keeps_greater_transposition(
        tt: TranspositionTable,
        t: Transposition,
        u: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        tt.set(k, u);

        if t > u {
            assert_eq!(tt.get(k), Some(t));
        } else {
            assert_eq!(tt.get(k), Some(u));
        }
    }

    #[proptest]
    fn set_ignores_the_signature_mismatch(
        tt: TranspositionTable,
        t: Transposition,
        #[filter(#u.draft() > #t.draft())] u: Transposition,
        k: Zobrist,
    ) {
        let sig = tt.signature_of((!k.load::<u64>()).view_bits().into());
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
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
