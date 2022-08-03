use serde::{Deserialize, Serialize};
use std::{fmt::Debug, time::Duration};

/// Configuration for the limits of search engines.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[serde(deny_unknown_fields, rename = "limits", default)]
pub struct SearchLimits {
    /// The maximum number of plies to search.
    pub depth: u8,

    /// The maximum amount of time to spend searching.
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
