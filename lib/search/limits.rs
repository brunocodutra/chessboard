use super::Depth;
use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use test_strategy::Arbitrary;

/// Configuration for search limits.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Arbitrary, From, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Limits {
    /// Unlimited search.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    None,

    /// The maximum number of plies to search.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    Depth(Depth),

    /// The maximum amount of time to spend searching.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    #[serde(with = "humantime_serde")]
    Time(Duration),
}

impl Default for Limits {
    fn default() -> Self {
        Limits::None
    }
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
    /// Depth or [`Depth::upper()`].
    pub fn depth(&self) -> Depth {
        match self {
            Limits::Depth(d) => *d,
            _ => Depth::upper(),
        }
    }

    /// Time or [`Duration::MAX`].
    pub fn time(&self) -> Duration {
        match self {
            Limits::Time(t) => *t,
            _ => Duration::MAX,
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
    fn depth_returns_max_by_default(d: Duration) {
        assert_eq!(Limits::None.depth(), Depth::upper());
        assert_eq!(Limits::Time(d).depth(), Depth::upper());
    }

    #[proptest]
    fn time_returns_value_if_set(d: Duration) {
        assert_eq!(Limits::Time(d).time(), d);
    }

    #[proptest]
    fn time_returns_max_by_default(d: Depth) {
        assert_eq!(Limits::None.time(), Duration::MAX);
        assert_eq!(Limits::Depth(d).time(), Duration::MAX);
    }
}
