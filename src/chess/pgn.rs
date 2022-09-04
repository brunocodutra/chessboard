use super::{Color, Outcome, San};
use std::fmt::{self, Display};

/// The description of a chess game.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Pgn {
    pub white: String,
    pub black: String,
    pub outcome: Outcome,
    pub moves: Vec<San>,
}

/// Prints a simplified [PGN] description of the game
///
/// [PGN]: https://www.chessprogramming.org/Portable_Game_Notation
impl Display for Pgn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[White {:?}]", self.white)?;
        writeln!(f, "[Black {:?}]", self.black)?;

        for (i, san) in self.moves.iter().enumerate() {
            if i % 2 == 0 {
                write!(f, "{}. ", i / 2 + 1)?;
            }

            write!(f, "{} ", san)?;
        }

        match self.outcome {
            Outcome::DrawBy75MoveRule => write!(f, "{{75-move rule}} 1/2-1/2"),
            Outcome::DrawByInsufficientMaterial => write!(f, "{{insufficient material}} 1/2-1/2"),
            Outcome::Stalemate => write!(f, "{{stalemate}} 1/2-1/2"),
            Outcome::Checkmate(Color::Black) => write!(f, "0-1"),
            Outcome::Checkmate(Color::White) => write!(f, "1-0"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgn_reader::{BufferedReader, Visitor};
    use shakmaty as sm;
    use std::mem::take;
    use test_strategy::proptest;

    #[derive(Default)]
    struct PgnVisitor {
        moves: Vec<San>,
    }

    impl Visitor for PgnVisitor {
        type Result = Vec<San>;

        fn san(&mut self, sp: sm::san::SanPlus) {
            self.moves.push(sp.san.into());
        }

        fn end_game(&mut self) -> Self::Result {
            take(&mut self.moves)
        }
    }

    #[proptest(cases = 10)]
    fn prints_simplified_pgn(pgn: Pgn) {
        let mut reader = BufferedReader::new_cursor(pgn.to_string());
        let mut visitor = PgnVisitor::default();
        assert_eq!(reader.read_game(&mut visitor)?, Some(pgn.moves));
    }
}
