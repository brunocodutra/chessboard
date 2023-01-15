use super::{Iter, OptionalSignedTransposition, Signature, Transposition};
use crate::chess::{Position, Zobrist};
use crate::util::{Binary, Cache};
use proptest::{collection::*, prelude::*};
use std::mem::size_of;
use test_strategy::Arbitrary;

/// A cache for [`Transposition`]s.
#[derive(Debug, Arbitrary)]
pub struct Table {
    #[strategy((1usize..=32, hash_map(any::<Position>(), any::<Transposition>(), 0..=32)).prop_map(|(cap, ts)| {
        let cache = Cache::new(cap.next_power_of_two());

        for (pos, t) in ts {
            let key = pos.zobrist();
            let idx = key.slice(..cache.len().trailing_zeros()).get() as _;
            let sig = key.slice(cache.len().trailing_zeros()..).pop();
            cache.store(idx, Some((t, sig)).encode());
        }

        cache
    })
    .no_shrink())]
    cache: Cache<<OptionalSignedTransposition as Binary>::Bits>,
}

impl Table {
    /// Constructs a transposition [`Table`] of at most `size` many bytes.
    ///
    /// The `size` specifies an upper bound, as long as the table is not empty.
    pub fn new(size: usize) -> Self {
        let entry_size = size_of::<<OptionalSignedTransposition as Binary>::Bits>();
        let cache_size = (size / entry_size + 1).next_power_of_two() / 2;

        Table {
            cache: Cache::new(cache_size.max(1)),
        }
    }

    /// The actual size of this [`Table`] in bytes.
    pub fn size(&self) -> usize {
        self.capacity() * size_of::<<OptionalSignedTransposition as Binary>::Bits>()
    }

    /// The actual size of this [`Table`] in number of entries.
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
        let sig = self.signature_of(key);
        let bits = self.cache.load(self.index_of(key));
        match Binary::decode(bits).expect("expected valid encoding") {
            Some((t, s)) if s == sig => Some(t),
            _ => None,
        }
    }

    /// Stores a [`Transposition`] in the slot associated with `key`.
    ///
    /// In the slot if not empty, the [`Transposition`] with greater depth is chosen.
    pub fn set(&self, key: Zobrist, transposition: Transposition) {
        let sig = self.signature_of(key);
        let bits = Some((transposition, sig)).encode();
        self.cache.update(self.index_of(key), |r| {
            match Binary::decode(r).expect("expected valid encoding") {
                Some((t, _)) if t > transposition => None,
                _ => Some(bits),
            }
        })
    }

    /// Clears the [`Transposition`] from the slot associated with `key`.
    pub fn unset(&self, key: Zobrist) {
        self.cache.store(self.index_of(key), None.encode())
    }

    /// An iterator for the principal variation from a starting [`Position`].
    pub fn iter(&self, pos: &Position) -> Iter<'_> {
        Iter::new(self, pos.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::Bits;
    use test_strategy::proptest;

    #[proptest]
    fn size_returns_table_capacity_in_bytes(tt: Table) {
        assert_eq!(
            tt.size(),
            tt.cache.len() * size_of::<<OptionalSignedTransposition as Binary>::Bits>()
        );
    }

    #[proptest]
    fn input_size_is_an_upper_limit(
        #[strategy(size_of::<<OptionalSignedTransposition as Binary>::Bits>()..=1024)] s: usize,
    ) {
        assert!(Table::new(s).size() <= s);
    }

    #[proptest]
    fn size_is_exact_if_input_is_power_of_two(
        #[strategy(size_of::<<OptionalSignedTransposition as Binary>::Bits>()..=1024)] s: usize,
    ) {
        assert_eq!(
            Table::new(s.next_power_of_two()).size(),
            s.next_power_of_two()
        );
    }

    #[proptest]
    fn capacity_returns_cache_len(tt: Table) {
        assert_eq!(tt.capacity(), tt.cache.len());
    }

    #[proptest]
    fn get_returns_none_if_transposition_does_not_exist(tt: Table, k: Zobrist) {
        tt.cache.store(tt.index_of(k), Bits::default());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_none_if_signature_does_not_match(tt: Table, t: Transposition, k: Zobrist) {
        let sig = !tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn get_returns_some_if_transposition_exists(tt: Table, t: Transposition, k: Zobrist) {
        let sig = tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn set_keeps_greater_transposition(tt: Table, t: Transposition, u: Transposition, k: Zobrist) {
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
        #[by_ref] tt: Table,
        t: Transposition,
        #[filter(#u.depth() > #t.depth())] u: Transposition,
        k: Zobrist,
    ) {
        let sig = !tt.signature_of(k);
        tt.cache.store(tt.index_of(k), Some((t, sig)).encode());
        tt.set(k, u);
        assert_eq!(tt.get(k), Some(u));
    }

    #[proptest]
    fn set_stores_transposition_if_none_exists(tt: Table, t: Transposition, k: Zobrist) {
        tt.cache.store(tt.index_of(k), Bits::default());
        tt.set(k, t);
        assert_eq!(tt.get(k), Some(t));
    }

    #[proptest]
    fn unset_erases_transposition(tt: Table, k: Zobrist) {
        tt.unset(k);
        assert_eq!(tt.get(k), None);
    }

    #[proptest]
    fn clear_resets_cache(mut tt: Table, k: Zobrist) {
        tt.clear();
        assert_eq!(tt.get(k), None);
    }
}
