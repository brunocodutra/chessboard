use crate::search::Depth;
use derive_more::From;
use std::time::Duration;

/// Configuration for search limits.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum Limits {
    /// Unlimited search.
    #[default]
    None,

    /// The maximum number of plies to search.
    Depth(Depth),

    /// The maximum amount of time to spend searching.
    Time(Duration),

    /// The time remaining on the clock.
    #[from(ignore)]
    Clock(Duration, Duration),
}

impl Limits {
    /// Maximum depth or [`Depth::UPPER`].
    pub fn depth(&self) -> Depth {
        match self {
            Limits::Depth(d) => *d,
            _ => Depth::UPPER,
        }
    }

    /// Maximum time or [`Duration::MAX`].
    pub fn time(&self) -> Duration {
        match self {
            Limits::Time(t) => *t,
            Limits::Clock(t, _) => *t,
            _ => Duration::MAX,
        }
    }

    /// Time left on the clock or [`Duration::MAX`].
    pub fn clock(&self) -> Duration {
        match self {
            Limits::Clock(t, _) => *t,
            _ => Duration::MAX,
        }
    }

    /// Time increment or [`Duration::ZERO`].
    pub fn increment(&self) -> Duration {
        match self {
            Limits::Clock(_, i) => *i,
            _ => Duration::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use test_strategy::proptest;

    #[proptest]
    fn depth_returns_value_if_set(d: Depth) {
        assert_eq!(Limits::Depth(d).depth(), d);
    }

    #[proptest]
    fn depth_returns_max_by_default(t: Duration, i: Duration) {
        assert_eq!(Limits::None.depth(), Depth::UPPER);
        assert_eq!(Limits::Time(t).depth(), Depth::UPPER);
        assert_eq!(Limits::Clock(t, i).depth(), Depth::UPPER);
    }

    #[proptest]
    fn time_returns_value_if_set(t: Duration) {
        assert_eq!(Limits::Time(t).time(), t);
    }

    #[proptest]
    fn time_returns_max_or_clock_by_default(d: Depth, t: Duration, i: Duration) {
        assert_eq!(Limits::None.time(), Duration::MAX);
        assert_eq!(Limits::Depth(d).time(), Duration::MAX);
        assert_eq!(Limits::Clock(t, i).time(), t);
    }

    #[proptest]
    fn clock_returns_value_if_set(t: Duration, i: Duration) {
        assert_eq!(Limits::Clock(t, i).clock(), t);
    }

    #[proptest]
    fn clock_returns_max_by_default(d: Depth, t: Duration) {
        assert_eq!(Limits::None.clock(), Duration::MAX);
        assert_eq!(Limits::Depth(d).clock(), Duration::MAX);
        assert_eq!(Limits::Time(t).clock(), Duration::MAX);
    }

    #[proptest]
    fn increment_returns_value_if_set(t: Duration, i: Duration) {
        assert_eq!(Limits::Clock(t, i).increment(), i);
    }

    #[proptest]
    fn increment_returns_zero_by_default(d: Depth, t: Duration) {
        assert_eq!(Limits::None.increment(), Duration::ZERO);
        assert_eq!(Limits::Depth(d).increment(), Duration::ZERO);
        assert_eq!(Limits::Time(t).increment(), Duration::ZERO);
    }
}
