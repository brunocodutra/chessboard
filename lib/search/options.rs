use derive_more::{Display, Error, From};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroUsize, str::FromStr};
use test_strategy::Arbitrary;

/// Configuration for adversarial search algorithms.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Arbitrary, Deserialize, Serialize)]
#[display(fmt = "{}", "ron::ser::to_string(self).unwrap()")]
#[serde(deny_unknown_fields, rename = "options", default)]
pub struct Options {
    /// The size of the transposition table in bytes.
    ///
    /// This is an upper limit, the actual memory allocation may be smaller.
    #[strategy(0usize..=1024)]
    pub hash: usize,

    /// The number of threads to use while searching.
    #[strategy((1usize..=4).prop_filter_map("zero", |t| NonZeroUsize::new(t)))]
    pub threads: NonZeroUsize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            hash: 1 << 25,
            threads: NonZeroUsize::new(1).unwrap(),
        }
    }
}

/// The reason why parsing [`Options`] failed.
#[derive(Debug, Display, Eq, PartialEq, Error, From)]
#[display(fmt = "failed to parse minimax configuration")]
pub struct ParseOptionsError(ron::de::SpannedError);

impl FromStr for Options {
    type Err = ParseOptionsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ron::de::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn options_deserializes_missing_fields_to_default() {
        assert_eq!("options()".parse(), Ok(Options::default()));
    }

    #[proptest]
    fn parsing_printed_options_is_an_identity(o: Options) {
        assert_eq!(o.to_string().parse(), Ok(o));
    }
}
