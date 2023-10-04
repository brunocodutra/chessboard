use std::num::NonZeroUsize;

#[cfg(test)]
use proptest::prelude::*;

/// Configuration for adversarial search algorithms.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Options {
    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[cfg_attr(test, strategy(..=1024usize))]
    pub hash: usize,

    /// The number of threads to use while searching.
    #[cfg_attr(test, strategy((1..=4usize).prop_filter_map("zero", |t| NonZeroUsize::new(t))))]
    pub threads: NonZeroUsize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hash: 16 << 20,
            threads: NonZeroUsize::new(1).unwrap(),
        }
    }
}
