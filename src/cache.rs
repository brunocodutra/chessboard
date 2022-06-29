use crate::Binary;
use atomic::{Atomic, Ordering};
use std::error::Error;

#[derive(Debug)]
/// A fixed-size concurrent in-memory cache.
pub struct Cache<T: Default + Binary> {
    memory: Vec<Atomic<T::Register>>,
}

impl<T> Cache<T>
where
    T: Default + Binary,
    T::Error: Error,
{
    /// Constructs a [`Cache`] with `size` many slots filled with `T::default()`.
    pub fn new(size: usize) -> Self {
        debug_assert!(Atomic::<T::Register>::is_lock_free());
        let bits = T::default().encode();
        let memory = (0..size).map(|_| Atomic::new(bits)).collect();
        Cache { memory }
    }

    /// The [`Cache`] size.
    #[inline]
    pub fn len(&self) -> usize {
        self.memory.len()
    }

    /// Whether the [`Cache`] is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    /// Loads a value from the cache.
    pub fn load(&self, idx: usize) -> T {
        T::decode(self.memory[idx].load(Ordering::Relaxed)).expect("expected valid encoding")
    }

    /// Stores a value in the cache.
    pub fn store(&self, idx: usize, value: T) {
        self.memory[idx].store(value.encode(), Ordering::Relaxed);
    }

    /// Updates a value in the cache.
    ///
    /// The operation is aborted if `value` returns `None`.
    pub fn update(&self, idx: usize, value: impl Fn(T) -> Option<T>) {
        let slot = &self.memory[idx];
        let mut old = slot.load(Ordering::Relaxed);
        while let Some(v) = value(T::decode(old).expect("expected valid encoding")) {
            let new = v.encode();
            match slot.compare_exchange_weak(old, new, Ordering::Relaxed, Ordering::Relaxed) {
                Err(current) => old = current,
                _ => break,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Bits;
    use proptest::collection::size_range;
    use rayon::prelude::*;
    use test_strategy::proptest;

    #[proptest]
    fn new_initializes_cache(#[strategy(1..=100usize)] s: usize, #[strategy(0..#s)] i: usize) {
        let cache = Cache::<Bits<u64, 48>>::new(s);
        assert_eq!(
            cache.memory[i].load(Ordering::SeqCst),
            Bits::<u64, 48>::default()
        );
    }

    #[proptest]
    fn len_returns_cache_size(#[strategy(0..=100usize)] s: usize) {
        assert_eq!(Cache::<Bits<u64, 48>>::new(s).len(), s);
    }

    #[proptest]
    fn is_empty_returns_whether_cache_is_disabled(#[strategy(0..=100usize)] s: usize) {
        assert_eq!(Cache::<Bits<u64, 48>>::new(s).is_empty(), s == 0);
    }

    #[proptest]
    fn load_reads_value_at_index(#[strategy(1..=100usize)] s: usize, #[strategy(0..#s)] i: usize) {
        let cache = Cache::<Bits<u64, 48>>::new(s);
        assert_eq!(cache.load(i), cache.memory[i].load(Ordering::SeqCst));
    }

    #[proptest]
    fn store_writes_value_at_index(
        #[strategy(1..=100usize)] s: usize,
        #[strategy(0..#s)] i: usize,
        v: Bits<u64, 48>,
    ) {
        let cache = Cache::<Bits<u64, 48>>::new(s);
        cache.store(i, v);
        assert_eq!(cache.memory[i].load(Ordering::SeqCst), v);
    }

    #[proptest]
    fn update_writes_value_at_index_if_supplier_returns_some(
        #[strategy(1..=100usize)] s: usize,
        #[strategy(0..#s)] i: usize,
        v: Bits<u64, 48>,
    ) {
        let cache = Cache::<Bits<u64, 48>>::new(s);
        cache.update(i, |_| Some(v));
        assert_eq!(cache.memory[i].load(Ordering::SeqCst), v);
    }

    #[proptest]
    fn update_aborts_if_supplier_returns_none(
        #[strategy(1..=100usize)] s: usize,
        #[strategy(0..#s)] i: usize,
    ) {
        let cache = Cache::<Bits<u64, 48>>::new(s);
        cache.update(i, |_| None);
        assert_eq!(
            cache.memory[i].load(Ordering::SeqCst),
            Bits::<u64, 48>::default()
        );
    }

    #[proptest]
    fn cache_is_thread_safe(#[any(size_range(0..=100).lift())] vs: Vec<Bits<u64, 48>>) {
        let cache = Cache::<Bits<u64, 48>>::new(vs.len());

        vs.par_iter().enumerate().for_each(|(i, v)| {
            cache.store(i, *v);
        });

        vs.into_par_iter().enumerate().for_each(|(i, v)| {
            assert_eq!(cache.load(i), v);
        });
    }
}
