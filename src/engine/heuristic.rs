use crate::{Eval, Piece, Position, Role};

/// An engine that evaluates positions based on heuristics.
#[derive(Debug, Default, Clone)]
pub struct Heuristic {}

impl Heuristic {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Eval for Heuristic {
    fn eval(&self, pos: &Position) -> i32 {
        if pos.is_draw() || pos.is_stalemate() {
            0
        } else if pos.is_checkmate() {
            i32::MIN
        } else {
            let c = pos.turn();
            let p = pos.placement();

            // Fisher's system
            [
                (Role::Pawn, 100),
                (Role::Knight, 300),
                (Role::Bishop, 325),
                (Role::Rook, 500),
                (Role::Queen, 900),
            ]
            .iter()
            .map(|&(r, s)| (p.pieces(Piece(c, r)) as i32 - p.pieces(Piece(!c, r)) as i32) * s)
            .sum()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PositionKind;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(pos: Position) {
        assert_eq!(
            Heuristic::new().eval(&pos),
            Heuristic::new().eval(&pos.clone())
        );
    }

    #[proptest]
    fn draw_position_evaluates_to_balanced_score(#[any(PositionKind::Stalemate)] pos: Position) {
        assert_eq!(Heuristic::new().eval(&pos), 0);
    }

    #[proptest]
    fn stalemate_position_evaluates_to_balanced_score(
        #[any(PositionKind::Stalemate)] pos: Position,
    ) {
        assert_eq!(Heuristic::new().eval(&pos), 0);
    }

    #[proptest]
    fn checkmate_position_evaluates_to_lowest_possible_score(
        #[any(PositionKind::Checkmate)] pos: Position,
    ) {
        assert_eq!(Heuristic::new().eval(&pos), i32::MIN);
    }
}
