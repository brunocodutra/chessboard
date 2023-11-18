use derive_more::{Display, Error};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Error)]
#[display("counter exhausted")]
pub struct Exhausted;

/// A counter towards a limit.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Counter {
    #[cfg_attr(test, map(AtomicU64::new))]
    remaining: AtomicU64,
}

impl Counter {
    /// Constructs a counter with the given limit.
    pub fn new(limit: u64) -> Self {
        Counter {
            remaining: AtomicU64::new(limit),
        }
    }

    /// Increments the counter and returns the counts remaining if any.
    pub fn count(&self) -> Option<u64> {
        self.remaining
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |c| c.checked_sub(1))
            .map_or(None, |c| Some(c - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn counter_measures_time_remaining(#[strategy(1u64..)] c: u64) {
        let counter = Counter::new(c);
        assert_eq!(counter.count(), Some(c - 1));
    }

    #[proptest]
    fn counter_overflows_once_limit_is_reached() {
        let counter = Counter::new(0);
        assert_eq!(counter.count(), None);
    }
}
