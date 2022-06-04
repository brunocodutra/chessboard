use derive_more::{DebugCustom, Display, Error, From};
use shakmaty as sm;
use std::str::FromStr;

#[cfg(test)]
use proptest::{prelude::*, sample::Selector};

/// A representation of the [algebraic notation].
///
/// [algebraic notation]: https://en.wikipedia.org/wiki/Algebraic_notation_(chess)
#[derive(DebugCustom, Display, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[debug(fmt = "San(\"{}\")", self)]
#[display(fmt = "{}", _0)]
pub struct San(
    #[cfg_attr(test, strategy(
        (any::<crate::Position>(), any::<Selector>()).prop_filter_map("end position", |(pos, selector)| {
            let chess: sm::Chess = pos.into();
            let m = selector.try_select(sm::Position::legal_moves(&chess))?;
            Some(sm::san::SanPlus::from_move(chess, &m))
        })
    ))]
    sm::san::SanPlus,
);

impl San {
    /// The null move, used to indicate the player's resignation.
    pub fn null() -> Self {
        San(sm::san::SanPlus {
            san: sm::san::San::Null,
            suffix: None,
        })
    }
}

/// The reason why the string is not valid FEN.
#[derive(Debug, Display, Clone, Error, From)]
#[display(fmt = "{}", _0)]
pub struct ParseSanError(#[error(not(source))] sm::san::ParseSanError);

impl FromStr for San {
    type Err = ParseSanError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(San(s.parse()?))
    }
}

#[doc(hidden)]
impl From<sm::san::SanPlus> for San {
    fn from(san: sm::san::SanPlus) -> Self {
        San(san)
    }
}

#[doc(hidden)]
impl From<San> for sm::san::SanPlus {
    fn from(san: San) -> Self {
        san.0
    }
}

#[doc(hidden)]
impl AsRef<sm::san::SanPlus> for San {
    fn as_ref(&self) -> &sm::san::SanPlus {
        &self.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn parsing_printed_san_is_an_identity(san: San) {
        assert_eq!(san.to_string().parse().ok(), Some(san));
    }

    #[proptest]
    fn parsing_invalid_san_fails(
        #[by_ref]
        #[filter(#s.parse::<sm::san::SanPlus>().is_err())]
        s: String,
    ) {
        assert!(s.parse::<San>().is_err());
    }
}
