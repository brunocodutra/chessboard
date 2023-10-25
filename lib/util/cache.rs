use atomic::{Atomic, Ordering};
use bytemuck::NoUninit;

#[cfg(test)]
use proptest::{collection::*, prelude::*};

#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = SizeRange, bound(T)))]
/// A fixed-size concurrent in-memory cache.
pub struct Cache<T: NoUninit> {
    #[cfg_attr(test, strategy(vec(any::<T>().prop_map(|v| Atomic::new(v)), (*args).clone())))]
    memory: Vec<Atomic<T>>,
}

impl<T: NoUninit + Default> Cache<T> {
    /// Constructs a [`Cache`] with `size` many slots filled with `T::default()`.
    pub fn new(size: usize) -> Self {
        debug_assert!(Atomic::<T>::is_lock_free());
        Cache {
            memory: (0..size).map(|_| Atomic::default()).collect(),
        }
    }

    /// Resets all elements in the cache to `T::default()`.
    pub fn clear(&mut self) {
        self.memory.fill_with(Atomic::default);
    }

    /// The [`Cache`] size.
    pub fn len(&self) -> usize {
        self.memory.len()
    }

    /// Whether the [`Cache`] is empty.
    pub fn is_empty(&self) -> bool {
        self.memory.is_empty()
    }

    /// Loads a value from the cache.
    pub fn load(&self, idx: usize) -> T {
        self.memory[idx].load(Ordering::Relaxed)
    }

    /// Stores a value in the cache.
    pub fn store(&self, idx: usize, value: T) {
        self.memory[idx].store(value, Ordering::Relaxed);
    }

    /// Updates a value in the cache.
    ///
    /// The operation is aborted if `value` returns `None`.
    pub fn update(&self, idx: usize, value: impl Fn(T) -> Option<T>) {
        let slot = &self.memory[idx];
        let mut old = slot.load(Ordering::Relaxed);
        while let Some(new) = value(old) {
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
    use proptest::collection::size_range;
    use rayon::prelude::*;
    use std::sync::Arc;
    use test_strategy::proptest;

    #[proptest]
    fn new_initializes_cache(#[strategy(1..=100usize)] s: usize, #[strategy(0..#s)] i: usize) {
        assert_eq!(Cache::<u64>::new(s).memory[i].load(Ordering::SeqCst), 0);
    }

    #[proptest]
    fn clear_resets_cache(mut c: Cache<u64>) {
        let d = Cache::<u64>::new(c.len());
        c.clear();
        assert_eq!(
            c.memory
                .into_iter()
                .map(Atomic::into_inner)
                .collect::<Vec<_>>(),
            d.memory
                .into_iter()
                .map(Atomic::into_inner)
                .collect::<Vec<_>>(),
        );
    }

    #[proptest]
    fn len_returns_cache_size(#[strategy(0..=100usize)] s: usize) {
        assert_eq!(Cache::<u64>::new(s).len(), s);
    }

    #[proptest]
    fn is_empty_returns_whether_cache_is_disabled(#[strategy(0..=100usize)] s: usize) {
        assert_eq!(Cache::<u64>::new(s).is_empty(), s == 0);
    }

    #[proptest]
    fn load_reads_value_at_index(
        #[any((1..100).into())] c: Arc<Cache<u64>>,
        #[strategy(0..#c.len())] i: usize,
    ) {
        assert_eq!(c.load(i), c.memory[i].load(Ordering::SeqCst));
    }

    #[proptest]
    fn store_writes_value_at_index(
        #[any((1..100).into())] c: Arc<Cache<u64>>,
        #[strategy(0..#c.len())] i: usize,
        v: u64,
    ) {
        c.store(i, v);
        assert_eq!(c.memory[i].load(Ordering::SeqCst), v);
    }

    #[proptest]
    fn update_writes_value_at_index_if_supplier_returns_some(
        #[any((1..100).into())] c: Arc<Cache<u64>>,
        #[strategy(0..#c.len())] i: usize,
        v: u64,
    ) {
        c.update(i, |_| Some(v));
        assert_eq!(c.memory[i].load(Ordering::SeqCst), v);
    }

    #[proptest]
    fn update_aborts_if_supplier_returns_none(
        #[any((1..100).into())] c: Arc<Cache<u64>>,
        #[strategy(0..#c.len())] i: usize,
    ) {
        let v = c.memory[i].load(Ordering::SeqCst);
        c.update(i, |_| None);
        assert_eq!(c.memory[i].load(Ordering::SeqCst), v);
    }

    #[proptest]
    fn cache_is_thread_safe(
        #[any((1..100).into())] c: Arc<Cache<u64>>,
        #[any(size_range(#c.len()).lift())] vs: Vec<u64>,
    ) {
        vs.par_iter().enumerate().for_each(|(i, v)| {
            c.store(i, *v);
        });

        vs.into_par_iter().enumerate().for_each(|(i, v)| {
            assert_eq!(c.load(i), v);
        });
    }
}
