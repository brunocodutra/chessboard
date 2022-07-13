use crate::engine::PieceSquareTable;
use derive_more::Constructor;

/// An engine that evaluates positions purely based on piece values.
#[derive(Debug, Default, Constructor)]
pub struct Materialist {}

impl PieceSquareTable for Materialist {
    const PIECE_VALUE: [i16; 6] = [100, 300, 300, 500, 900, 0];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Eval, Game, Outcome};
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(g: Game) {
        assert_eq!(
            Materialist::new().eval(&g),
            Materialist::new().eval(&g.clone())
        );
    }

    #[proptest]
    fn draw_evaluates_to_balanced_score(
        #[filter(#_o.is_draw())] _o: Outcome,
        #[any(Some(#_o))] g: Game,
    ) {
        assert_eq!(Materialist::new().eval(&g), 0);
    }

    #[proptest]
    fn lost_game_evaluates_to_lowest_possible_score(
        #[filter(#_o.is_decisive())] _o: Outcome,
        #[any(Some(#_o))]
        #[filter(#_o.winner() != Some(#g.position().turn()))]
        g: Game,
    ) {
        assert_eq!(Materialist::new().eval(&g), i16::MIN);
    }

    #[proptest]
    fn won_game_evaluates_to_highest_possible_score(
        #[filter(#_o.is_decisive())] _o: Outcome,
        #[any(Some(#_o))]
        #[filter(#_o.winner() == Some(#g.position().turn()))]
        g: Game,
    ) {
        assert_eq!(Materialist::new().eval(&g), i16::MAX);
    }
}
