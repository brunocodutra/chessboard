use super::Color;
use derive_more::Display;
use test_strategy::Arbitrary;

/// One of the possible outcomes of a chess game.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Arbitrary)]
pub enum Outcome {
    #[display(fmt = "checkmate by the {_0} player")]
    Checkmate(Color),

    #[display(fmt = "{_0} player lost on time")]
    LossOnTime(Color),

    #[display(fmt = "{_0} player resigned")]
    Resignation(Color),

    #[display(fmt = "stalemate")]
    Stalemate,

    #[display(fmt = "draw by insufficient material")]
    DrawByInsufficientMaterial,

    #[display(fmt = "draw by the 75-move rule")]
    DrawBy75MoveRule,
}

impl Outcome {
    /// Whether the outcome is a [draw] and neither side has won.
    ///
    /// [draw]: https://www.chessprogramming.org/Draw
    pub fn is_draw(&self) -> bool {
        !self.is_decisive()
    }

    /// Whether the outcome is a decisive and one of the sides has won.
    pub fn is_decisive(&self) -> bool {
        use Outcome::*;
        matches!(self, Checkmate(_) | LossOnTime(_) | Resignation(_))
    }

    /// The winning side, if the outcome is [decisive](`Self::is_decisive`).
    pub fn winner(&self) -> Option<Color> {
        match *self {
            Outcome::Checkmate(c) => Some(c),
            Outcome::LossOnTime(c) => Some(!c),
            Outcome::Resignation(c) => Some(!c),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn outcome_is_either_draw_or_decisive(o: Outcome) {
        assert_ne!(o.is_draw(), o.is_decisive());
    }

    #[proptest]
    fn neither_side_wins_if_draw(#[filter(#o.is_draw())] o: Outcome) {
        assert_eq!(o.winner(), None);
    }

    #[proptest]
    fn one_side_wins_if_decisive(#[filter(#o.is_decisive())] o: Outcome) {
        assert_ne!(o.winner(), None);
    }

    #[proptest]
    fn side_that_checkmates_wins(c: Color) {
        assert_eq!(Outcome::Checkmate(c).winner(), Some(c));
    }

    #[proptest]
    fn side_that_runs_out_of_time_loses(c: Color) {
        assert_eq!(Outcome::LossOnTime(c).winner(), Some(!c));
    }

    #[proptest]
    fn side_that_resigns_loses(c: Color) {
        assert_eq!(Outcome::Resignation(c).winner(), Some(!c));
    }
}
