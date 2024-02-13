use crate::util::Integer;
use derive_more::{Debug, Deref, Display, Error, Shl, Shr};
use std::{cmp::Ordering, str::FromStr};

/// The hash size in bytes.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref, Shl, Shr)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("HashSize({_0})")]
#[display("{}", self.get() >> 20)]
#[repr(transparent)]
pub struct HashSize(#[cfg_attr(test, strategy(..=Self::MAX))] usize);

impl HashSize {
    /// Constructs hash size.
    pub fn new(size: usize) -> Self {
        debug_assert!((Self::MIN..=Self::MAX).contains(&size));
        Self::from_repr(size)
    }

    /// Returns the raw hash size.
    pub fn get(&self) -> usize {
        self.repr()
    }
}

unsafe impl const Integer for HashSize {
    type Repr = usize;

    const MIN: Self::Repr = 0;

    #[cfg(not(test))]
    const MAX: usize = match 1usize.checked_shl(45) {
        Some(h) => h,
        None => usize::MAX,
    };

    #[cfg(test)]
    const MAX: usize = 16 << 20;
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

impl PartialEq<HashSize> for usize {
    fn eq(&self, other: &HashSize) -> bool {
        self.eq(&other.0)
    }
}

impl PartialOrd<HashSize> for usize {
    fn partial_cmp(&self, other: &HashSize) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}
/// The reason why parsing the hash size failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    "expected integer in the range `({}..={})`",
    HashSize::lower(),
    HashSize::upper()
)]
pub struct ParseHashSizeError;

impl FromStr for HashSize {
    type Err = ParseHashSizeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<usize>().ok().and_then(|h| h.checked_shl(20)) {
            Some(h) if (HashSize::MIN..=HashSize::MAX).contains(&h) => Ok(HashSize::new(h)),
            _ => Err(ParseHashSizeError),
        }
    }
}

/// The thread count.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug("ThreadCount({_0})")]
#[display("{_0}")]
#[repr(transparent)]
pub struct ThreadCount(#[cfg_attr(test, strategy((1..=4usize)))] usize);

impl ThreadCount {
    /// Constructs hash size.
    pub fn new(count: usize) -> Self {
        debug_assert!((Self::MIN..=Self::MAX).contains(&count));
        Self::from_repr(count)
    }

    /// Returns the raw thread count.
    pub fn get(&self) -> usize {
        self.repr()
    }
}

unsafe impl const Integer for ThreadCount {
    type Repr = usize;
    const MIN: Self::Repr = 1;
    const MAX: Self::Repr = 1 << 16;
}

impl Default for ThreadCount {
    fn default() -> Self {
        Self::new(1)
    }
}

impl PartialEq<usize> for ThreadCount {
    fn eq(&self, other: &usize) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<usize> for ThreadCount {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl PartialEq<ThreadCount> for usize {
    fn eq(&self, other: &ThreadCount) -> bool {
        self.eq(&other.0)
    }
}

impl PartialOrd<ThreadCount> for usize {
    fn partial_cmp(&self, other: &ThreadCount) -> Option<Ordering> {
        self.partial_cmp(&other.0)
    }
}

/// The reason why parsing the thread count failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    "expected integer in the range `({}..={})`",
    ThreadCount::lower(),
    ThreadCount::upper()
)]
pub struct ParseThreadCountError;

impl FromStr for ThreadCount {
    type Err = ParseThreadCountError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<usize>() {
            Ok(t) if (ThreadCount::MIN..=ThreadCount::MAX).contains(&t) => Ok(ThreadCount::new(t)),
            _ => Err(ParseThreadCountError),
        }
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
    use std::fmt::Debug;
    use test_strategy::proptest;

    #[proptest]
    fn hash_size_is_smaller_than_max(h: HashSize) {
        assert!(HashSize::default() <= HashSize::MAX);
        assert!(h <= HashSize::MAX);
    }

    #[proptest]
    fn hash_size_constructs_if_size_not_too_large(
        #[strategy(HashSize::MIN..=HashSize::MAX)] n: usize,
    ) {
        assert_eq!(HashSize::new(n), n);
    }

    #[proptest]
    #[should_panic]
    fn hash_size_panics_if_size_too_large(#[strategy(HashSize::MAX + 1..)] n: usize) {
        HashSize::new(n);
    }

    #[proptest]
    fn parsing_printed_hash_size_rounds_to_megabytes(h: HashSize) {
        assert_eq!(h.to_string().parse(), Ok(h >> 20 << 20));
    }

    #[proptest]
    fn parsing_hash_size_fails_for_numbers_too_large(#[strategy(HashSize::MAX + 1..)] n: usize) {
        assert_eq!(n.to_string().parse::<HashSize>(), Err(ParseHashSizeError));
    }

    #[proptest]
    fn parsing_hash_size_fails_for_invalid_number(
        #[filter(#s.parse::<usize>().is_err())] s: String,
    ) {
        assert_eq!(s.to_string().parse::<HashSize>(), Err(ParseHashSizeError));
    }

    #[proptest]
    fn thread_count_is_smaller_than_max(t: ThreadCount) {
        assert!(ThreadCount::default() <= ThreadCount::MAX);
        assert!(t <= ThreadCount::MAX);
    }

    #[proptest]
    fn thread_count_constructs_if_count_not_too_large(
        #[strategy((ThreadCount::MIN..=ThreadCount::MAX))] n: usize,
    ) {
        assert_eq!(ThreadCount::new(n), n);
    }

    #[proptest]
    #[should_panic]
    fn thread_count_panics_if_count_too_large(#[strategy((ThreadCount::MAX + 1..))] n: usize) {
        ThreadCount::new(n);
    }

    #[proptest]
    fn parsing_printed_thread_count_is_an_identity(t: ThreadCount) {
        assert_eq!(t.to_string().parse(), Ok(t));
    }

    #[proptest]
    fn parsing_thread_count_fails_for_numbers_too_large(
        #[strategy(ThreadCount::MAX + 1..)] n: usize,
    ) {
        assert_eq!(
            n.to_string().parse::<ThreadCount>(),
            Err(ParseThreadCountError)
        );
    }

    #[proptest]
    fn parsing_thread_count_fails_for_invalid_number(
        #[filter(#s.parse::<usize>().is_err())] s: String,
    ) {
        assert_eq!(
            s.to_string().parse::<ThreadCount>(),
            Err(ParseThreadCountError)
        );
    }
}
