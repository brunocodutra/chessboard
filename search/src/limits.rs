use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use test_strategy::Arbitrary;
use util::Depth;

/// Configuration for search limits.
#[derive(
    Debug, Display, Default, Copy, Clone, Eq, PartialEq, Arbitrary, From, Deserialize, Serialize,
)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Limits {
    /// Unlimited search.
    #[default]
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    None,

    /// The maximum number of plies to search.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Depth(Depth),

    /// The maximum amount of time to spend searching.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Time(#[serde(with = "humantime_serde")] Duration),

    /// The time remaining on the clock.
    #[from(ignore)]
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Clock(
        #[serde(with = "humantime_serde")] Duration,
        #[serde(with = "humantime_serde", default = "no_increment")] Duration,
    ),
}

fn no_increment() -> Duration {
    Duration::ZERO
}

/// The reason why parsing [`Limits`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse minimax configuration")]
pub struct ParseLimitsError(ron::de::SpannedError);

impl FromStr for Limits {
    type Err = ParseLimitsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

impl Limits {
    /// Maximum depth or [`Depth::upper()`].
    #[inline]
    pub fn depth(&self) -> Depth {
        match self {
            Limits::Depth(d) => *d,
            _ => Depth::upper(),
        }
    }

    /// Maximum time or [`Duration::MAX`].
    #[inline]
    pub fn time(&self) -> Duration {
        match self {
            Limits::Time(t) => *t,
            Limits::Clock(t, _) => *t,
            _ => Duration::MAX,
        }
    }

    /// Time left on the clock or [`Duration::MAX`].
    #[inline]
    pub fn clock(&self) -> Duration {
        match self {
            Limits::Clock(t, _) => *t,
            _ => Duration::MAX,
        }
    }

    /// Time increment or [`Duration::ZERO`].
    #[inline]
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
    fn parsing_printed_search_limits_is_an_identity(l: Limits) {
        assert_eq!(l.to_string().parse(), Ok(l));
    }

    #[proptest]
    fn depth_returns_value_if_set(d: Depth) {
        assert_eq!(Limits::Depth(d).depth(), d);
    }

    #[proptest]
    fn depth_returns_max_by_default(t: Duration, i: Duration) {
        assert_eq!(Limits::None.depth(), Depth::upper());
        assert_eq!(Limits::Time(t).depth(), Depth::upper());
        assert_eq!(Limits::Clock(t, i).depth(), Depth::upper());
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
