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
        if pos.is_material_insufficient() || pos.is_stalemate() {
            0
        } else if pos.is_checkmate() {
            i32::MIN
        } else {
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
            .map(|(a, b, s)| (pos.pieces(a) as i32 - pos.pieces(b) as i32) * s)
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
    fn position_with_insufficient_material_evaluates_to_balanced_score(
        #[any(PositionKind::Stalemate)] pos: Position,
    ) {
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
