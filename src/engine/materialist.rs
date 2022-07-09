use crate::{Eval, Game, Piece, Role};
use derive_more::Constructor;

/// An engine that evaluates positions purely based on material.
#[derive(Debug, Default, Constructor)]
pub struct Materialist {}

impl Materialist {
    // Fisher's system
    const PIECE_VALUE: [i16; 5] = [100, 300, 325, 500, 900];
}

impl Eval for Materialist {
    fn eval(&self, game: &Game) -> i16 {
        let pos = game.position();
        let turn = pos.turn();

        match game.outcome() {
            Some(o) => match o.winner() {
                Some(w) if w == turn => i16::MAX,
                Some(_) => i16::MIN,
                None => 0,
            },

            None => {
                use Role::*;
                [Pawn, Knight, Bishop, Rook, Queen]
                    .into_iter()
                    .map(|r| {
                        let ours = pos.by_piece(Piece(turn, r)).len() as i16;
                        let theirs = pos.by_piece(Piece(!turn, r)).len() as i16;
                        (ours - theirs) * Self::PIECE_VALUE[r as usize]
                    })
                    .sum()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Outcome;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(g: Game) {
        let engine = Materialist::new();
        assert_eq!(engine.eval(&g), engine.eval(&g.clone()));
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
