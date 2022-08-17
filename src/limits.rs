use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(test)]
use proptest::prelude::*;

/// Configuration for the limits of search engines.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = (Option<u8>, Option<u8>)))]
#[serde(deny_unknown_fields, rename = "limits", default)]
pub struct SearchLimits {
    /// The maximum number of plies to search.
    #[cfg_attr(test, strategy(args.0.unwrap_or(u8::MIN)..=args.1.unwrap_or(u8::MAX)))]
    pub depth: u8,

    /// The maximum amount of time to spend searching.
    #[cfg_attr(test, strategy(Just(Duration::MAX)))]
    #[serde(with = "humantime_serde")]
    pub time: Duration,
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            depth: u8::MAX,
            time: Duration::MAX,
        }
    }
}
