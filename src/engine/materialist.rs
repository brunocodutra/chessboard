use super::PieceSquareTable;
use derive_more::Constructor;

/// An engine that evaluates positions purely based on piece values.
#[derive(Debug, Default, Clone, Constructor)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Materialist {}

impl PieceSquareTable for Materialist {
    const PIECE_VALUE: [i16; 6] = [100, 300, 300, 500, 900, 0];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chess::Position, Eval};
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(pos: Position) {
        assert_eq!(Materialist::new().eval(&pos), Materialist::new().eval(&pos));
    }
}
