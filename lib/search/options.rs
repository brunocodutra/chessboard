use crate::util::Assume;
use derive_more::{DebugCustom, Deref};
use rayon::max_num_threads;
use std::{cmp::Ordering, num::NonZeroUsize};

#[cfg(test)]
use proptest::prelude::*;

/// The hash size in bytes.
#[derive(DebugCustom, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "HashSize({_0})")]
pub struct HashSize(#[cfg_attr(test, strategy(..=Self::MAX))] usize);

impl HashSize {
    #[cfg(not(test))]
    const MAX: usize = match 1usize.checked_shl(45) {
        Some(h) => h,
        None => usize::MAX,
    };

    #[cfg(test)]
    const MAX: usize = 16 << 20;

    /// The maximum allowed hash size.
    pub fn max() -> Self {
        HashSize(Self::MAX)
    }

    /// Constructs hash size.
    ///
    /// # Panics
    ///
    /// Panics if size is too large.
    pub fn new(size: usize) -> Self {
        assert!(Self::max() >= size);
        HashSize(size)
    }
}

impl Default for HashSize {
    fn default() -> Self {
        HashSize(16 << 20)
    }
}

impl PartialEq<usize> for HashSize {
    fn eq(&self, other: &usize) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<usize> for HashSize {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

/// The thread count.
#[derive(DebugCustom, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "ThreadCount({_0})")]
pub struct ThreadCount(
    #[cfg_attr(test, strategy((1..=4usize).prop_map(|t| NonZeroUsize::new(t).assume())))]
    NonZeroUsize,
);

impl ThreadCount {
    /// The maximum allowed thread count.
    pub fn max() -> Self {
        ThreadCount(NonZeroUsize::new(max_num_threads()).assume())
    }

    /// Constructs hash size.
    ///
    /// # Panics
    ///
    /// Panics if count is too large.
    pub fn new(count: NonZeroUsize) -> Self {
        assert!(Self::max() >= count);
        ThreadCount(count)
    }
}

impl Default for ThreadCount {
    fn default() -> Self {
        ThreadCount::new(NonZeroUsize::new(1).assume())
    }
}

impl PartialEq<NonZeroUsize> for ThreadCount {
    fn eq(&self, other: &NonZeroUsize) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<NonZeroUsize> for ThreadCount {
    fn partial_cmp(&self, other: &NonZeroUsize) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<usize> for ThreadCount {
    fn eq(&self, other: &usize) -> bool {
        self.get().eq(other)
    }
}

impl PartialOrd<usize> for ThreadCount {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        self.get().partial_cmp(other)
    }
}

/// Configuration for adversarial search algorithms.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Options {
    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    pub hash: HashSize,

    /// The number of threads to use while searching.
    pub threads: ThreadCount,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn hash_size_is_smaller_than_max(h: HashSize) {
        assert!(HashSize::default() <= HashSize::max());
        assert!(h <= HashSize::max());
    }

    #[proptest]
    fn hash_size_constructs_if_size_not_too_large(#[strategy(..=HashSize::MAX)] s: usize) {
        assert_eq!(HashSize::new(s), s);
    }

    #[proptest]
    #[should_panic]
    fn hash_size_panics_if_size_too_large(#[strategy(HashSize::MAX + 1..)] s: usize) {
        HashSize::new(s);
    }

    #[proptest]
    fn thread_count_is_smaller_than_max(c: ThreadCount) {
        assert!(ThreadCount::default() <= ThreadCount::max());
        assert!(c <= ThreadCount::max());
    }

    #[proptest]
    fn thread_count_constructs_if_count_not_too_large(
        #[strategy((..=max_num_threads()).prop_map(|c| NonZeroUsize::new(c).unwrap()))]
        c: NonZeroUsize,
    ) {
        assert_eq!(ThreadCount::new(c), c);
    }

    #[proptest]
    #[should_panic]
    fn thread_count_panics_if_count_too_large(
        #[strategy((max_num_threads() + 1..).prop_map(|c| NonZeroUsize::new(c).unwrap()))]
        c: NonZeroUsize,
    ) {
        ThreadCount::new(c);
    }
}
