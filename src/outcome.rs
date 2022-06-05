use crate::Color;
use derive_more::Display;

/// One of the possible outcomes of a chess game.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum Outcome {
    #[display(fmt = "resignation by the {} player", _0)]
    Resignation(Color),

    #[display(fmt = "checkmate by the {} player", _0)]
    Checkmate(Color),

    #[display(fmt = "stalemate")]
    Stalemate,

    #[display(fmt = "draw by insufficient material")]
    DrawByInsufficientMaterial,
}

impl Outcome {
    /// Whether the outcome is a [draw] and neither side has won.
    ///
    /// [draw]: https://en.wikipedia.org/wiki/Glossary_of_chess#draw
    pub fn is_draw(&self) -> bool {
        use Outcome::*;
        matches!(self, Stalemate | DrawByInsufficientMaterial)
    }

    /// Whether the outcome is a decisive and one of the sides has won.
    pub fn is_decisive(&self) -> bool {
        use Outcome::*;
        matches!(self, Resignation(_) | Checkmate(_))
    }

    /// The winning side, if the outcome is [decisive](`is_decisive`).
    pub fn winner(&self) -> Option<Color> {
        match *self {
            Outcome::Checkmate(c) => Some(c),
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
    fn side_that_resigns_loses(c: Color) {
        assert_eq!(Outcome::Resignation(c).winner(), Some(!c));
    }
}
