use crate::chess::{Position, Promotion, Role};
use derive_more::Constructor;
use test_strategy::Arbitrary;

mod end;
mod mid;
mod pst;

pub use end::*;
pub use mid::*;
pub use pst::*;

/// Trait for types that can evaluate other types.
pub trait Eval<T> {
    /// Evaluates an item.
    ///
    /// Positive values favor the current side to play.
    fn eval(&self, item: &T) -> i16;
}

/// A tapered evaluator.
#[derive(Debug, Default, Clone, Arbitrary, Constructor)]
pub struct Evaluator();

impl Evaluator {
    const END_GAME: i32 = 518;
    const MID_GAME: i32 = 6192;
}

impl Eval<Position> for Evaluator {
    fn eval(&self, pos: &Position) -> i16 {
        let mg = MidGame.eval(pos) as i32;
        let eg = EndGame.eval(pos) as i32;

        use Role::*;
        let phase = [Knight, Bishop, Rook, Queen]
            .into_iter()
            .map(|r| pos.by_role(r).len() as i32 * MidGame.eval(&r) as i32)
            .sum::<i32>()
            .max(Self::END_GAME)
            .min(Self::MID_GAME);

        let score = eg + (mg - eg) * (phase - Self::END_GAME) / (Self::MID_GAME - Self::END_GAME);
        score.max(i16::MIN as i32).min(i16::MAX as i32) as i16
    }
}

impl Eval<Role> for Evaluator {
    fn eval(&self, role: &Role) -> i16 {
        EndGame.eval(role)
    }
}

impl Eval<Promotion> for Evaluator {
    fn eval(&self, p: &Promotion) -> i16 {
        EndGame.eval(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn score_is_stable(pos: Position) {
        assert_eq!(Evaluator::new().eval(&pos), Evaluator::new().eval(&pos));
    }

    #[proptest]
    fn starting_position_is_evaluated_as_mid_game() {
        assert_eq!(
            Evaluator::new().eval(&Position::default()),
            MidGame.eval(&Position::default())
        );
    }

    #[proptest]
    fn scores_are_tapered_between_mid_and_end_game(pos: Position) {
        let min = MidGame.eval(&pos).min(EndGame.eval(&pos));
        let max = MidGame.eval(&pos).max(EndGame.eval(&pos));
        assert!(Evaluator::new().eval(&pos) >= min);
        assert!(Evaluator::new().eval(&pos) <= max);
    }
}
