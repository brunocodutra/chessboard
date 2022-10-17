use super::Position;
use derive_more::{DebugCustom, Display};
use proptest::{prelude::*, sample::Selector};
use shakmaty as sm;
use test_strategy::Arbitrary;

/// A representation of the [algebraic notation].
///
/// [algebraic notation]: https://www.chessprogramming.org/Algebraic_Chess_Notation
#[derive(DebugCustom, Display, Clone, Eq, PartialEq, Hash, Arbitrary)]
#[debug(fmt = "San({})", self)]
#[display(fmt = "{}", _0)]
pub struct San(
    #[strategy(any::<(Position, Selector)>().prop_filter_map("end position", |(pos, selector)| {
            let m = selector.try_select(sm::Position::legal_moves(pos.as_ref()))?;
            Some(sm::san::San::from_move(pos.as_ref(), &m))
        })
    )]
    sm::san::San,
);

impl San {
    /// The null-move.
    pub fn null() -> Self {
        San(sm::san::San::Null)
    }
}

#[doc(hidden)]
impl From<sm::san::San> for San {
    fn from(san: sm::san::San) -> Self {
        San(san)
    }
}

#[doc(hidden)]
impl From<San> for sm::san::San {
    fn from(san: San) -> Self {
        san.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn san_has_an_equivalent_shakmaty_representation(san: San) {
        assert_eq!(San::from(sm::san::San::from(san.clone())), san);
    }
}
