use crate::{Eval, Game, Piece, Role};

/// An engine that evaluates positions based on heuristics.
#[derive(Debug, Default, Clone)]
pub struct Heuristic {}

impl Heuristic {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Eval for Heuristic {
    fn eval(&self, game: &Game) -> i32 {
        match game.outcome() {
            Some(o) => match o.winner() {
                Some(w) if w == game.position().turn() => i32::MAX,
                Some(_) => i32::MIN,
                None => 0,
            },

            None => {
                let pos = game.position();

                // Fisher's system
                [
                    (Role::Pawn, 100),
                    (Role::Knight, 300),
                    (Role::Bishop, 325),
                    (Role::Rook, 500),
                    (Role::Queen, 900),
                ]
                .into_iter()
                .map(|(r, s)| (Piece(pos.turn(), r), Piece(!pos.turn(), r), s))
                .map(|(a, b, s)| (pos.pieces(a).len() as i32 - pos.pieces(b).len() as i32) * s)
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
        let engine = Heuristic::new();
        assert_eq!(engine.eval(&g), engine.eval(&g.clone()));
    }

    #[proptest]
    fn draw_evaluates_to_balanced_score(
        #[filter(#_o.is_draw())] _o: Outcome,
        #[any(Some(#_o))] g: Game,
    ) {
        assert_eq!(Heuristic::new().eval(&g), 0);
    }

    #[proptest]
    fn lost_game_evaluates_to_lowest_possible_score(
        #[filter(#_o.is_decisive())] _o: Outcome,
        #[any(Some(#_o))]
        #[filter(#_o.winner() != Some(#g.position().turn()))]
        g: Game,
    ) {
        assert_eq!(Heuristic::new().eval(&g), i32::MIN);
    }

    #[proptest]
    fn won_game_evaluates_to_highest_possible_score(
        #[filter(#_o.is_decisive())] _o: Outcome,
        #[any(Some(#_o))]
        #[filter(#_o.winner() == Some(#g.position().turn()))]
        g: Game,
    ) {
        assert_eq!(Heuristic::new().eval(&g), i32::MAX);
    }
}
