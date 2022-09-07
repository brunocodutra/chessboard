use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, time::Duration};
use test_strategy::Arbitrary;

/// Configuration for search limits.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Arbitrary, Deserialize, Serialize)]
#[arbitrary(args = (Option<u8>, Option<u8>))]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
pub enum Limits {
    /// Unlimited search.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    None,

    /// The maximum number of plies to search.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    #[strategy(args.0.unwrap_or(u8::MIN)..=args.1.unwrap_or(u8::MAX))]
    Depth(u8),

    /// The maximum amount of time to spend searching.
    #[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
    #[strategy(Just(Duration::MAX))]
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
    /// Depth or [`u8::MAX`].
    pub fn depth(&self) -> u8 {
        match self {
            Limits::Depth(d) => *d,
            _ => u8::MAX,
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
    fn depth_returns_value_if_set(d: u8) {
        assert_eq!(Limits::Depth(d).depth(), d);
    }

    #[proptest]
    fn depth_returns_max_by_default(d: Duration) {
        assert_eq!(Limits::None.depth(), u8::MAX);
        assert_eq!(Limits::Time(d).depth(), u8::MAX);
    }

    #[proptest]
    fn time_returns_value_if_set(d: Duration) {
        assert_eq!(Limits::Time(d).time(), d);
    }

    #[proptest]
    fn time_returns_max_by_default(d: u8) {
        assert_eq!(Limits::None.time(), Duration::MAX);
        assert_eq!(Limits::Depth(d).time(), Duration::MAX);
    }
}
