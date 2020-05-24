use derive_more::{Display, Error};
use std::{fmt, iter::*, ops::*};

/// The color of a chess [Piece].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn to_str(self) -> &'static str {
        use Color::*;
        match self {
            White => &"white",
            Black => &"black",
        }
    }
}

/// A chess piece.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    pub fn to_str(self) -> &'static str {
        use Piece::*;
        match self {
            Pawn => &"pawn",
            Knight => &"knight",
            Bishop => &"bishop",
            Rook => &"rook",
            Queen => &"queen",
            King => &"king",
        }
    }
}

/// A chess piece of a certain color.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}", "self.to_str()")]
pub struct Figure {
    pub piece: Piece,
    pub color: Color,
}

impl Figure {
    pub fn to_str(self) -> &'static str {
        use Color::*;
        use Piece::*;
        match (self.piece, self.color) {
            (Pawn, White) => &"♙",
            (Knight, White) => &"♘",
            (Bishop, White) => &"♗",
            (Rook, White) => &"♖",
            (Queen, White) => &"♕",
            (King, White) => &"♔",
            (Pawn, Black) => &"♟",
            (Knight, Black) => &"♞",
            (Bishop, Black) => &"♝",
            (Rook, Black) => &"♜",
            (Queen, Black) => &"♛",
            (King, Black) => &"♚",
        }
    }
}

/// A column of the board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

impl File {
    pub const VARIANTS: &'static [File] = &[
        File::A,
        File::B,
        File::C,
        File::D,
        File::E,
        File::F,
        File::G,
        File::H,
    ];

    pub fn to_str(self) -> &'static str {
        use File::*;
        match self {
            A => &"a",
            B => &"b",
            C => &"c",
            D => &"d",
            E => &"e",
            F => &"f",
            G => &"g",
            H => &"h",
        }
    }
}

/// A row of the board.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Rank {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    Sixth,
    Seventh,
    Eighth,
}

impl Rank {
    pub const VARIANTS: &'static [Rank] = &[
        Rank::First,
        Rank::Second,
        Rank::Third,
        Rank::Fourth,
        Rank::Fifth,
        Rank::Sixth,
        Rank::Seventh,
        Rank::Eighth,
    ];

    pub fn to_str(self) -> &'static str {
        use Rank::*;
        match self {
            First => &"1",
            Second => &"2",
            Third => &"3",
            Fourth => &"4",
            Fifth => &"5",
            Sixth => &"6",
            Seventh => &"7",
            Eighth => &"8",
        }
    }
}

/// A square of the board.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}", "self.file.to_str()", "self.rank.to_str()")]
pub struct Square {
    pub file: File,
    pub rank: Rank,
}

/// A player by color.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Player {
    pub color: Color,
}

/// One of the possible outcomes of a chess game.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum Outcome {
    #[display(fmt = "resignation by the {} player", "_0.color.to_str()")]
    Resignation(Player),

    #[display(fmt = "checkmate by the {} player", "_0.color.to_str()")]
    Checkmate(Player),

    #[display(fmt = "stalemate")]
    Stalemate,

    #[display(fmt = "draw")]
    Draw,
}

/// A position on the board.
///
/// This type does not validate whether the position it holds is valid
/// according to any set of chess rules.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Position {
    pub squares: [[Option<Figure>; 8]; 8],
}

// We provide a custom implementation of Arbitrary rather than deriving,
// otherwise proptest overflows the stack generating large arrays.
#[cfg(test)]
impl proptest::arbitrary::Arbitrary for Position {
    type Parameters = ();
    type Strategy = proptest::prelude::BoxedStrategy<Position>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;

        vec![any::<Option<Figure>>(); 64]
            .prop_map(|v| {
                let mut squares: [[Option<Figure>; 8]; 8] = Default::default();
                squares
                    .iter_mut()
                    .flatten()
                    .zip(v)
                    .for_each(|(s, f)| *s = f);
                Position { squares }
            })
            .boxed()
    }
}

impl Index<Square> for Position {
    type Output = Option<Figure>;

    fn index(&self, s: Square) -> &Self::Output {
        &self.squares[s.rank as usize][s.file as usize]
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "   ")?;

        for &file in File::VARIANTS {
            write!(f, "  {} ", file.to_str())?;
        }

        writeln!(f)?;
        writeln!(f, "   +---+---+---+---+---+---+---+---+")?;
        for (&rank, row) in Rank::VARIANTS.iter().zip(&self.squares).rev() {
            write!(f, " {} |", rank.to_str())?;

            for &figure in row {
                write!(f, " {} |", figure.map(Figure::to_str).unwrap_or(" "))?;
            }
            writeln!(f, " {}", rank.to_str())?;
            writeln!(f, "   +---+---+---+---+---+---+---+---+")?;
        }

        write!(f, "   ")?;
        for &file in File::VARIANTS {
            write!(f, "  {} ", file.to_str())?;
        }

        Ok(())
    }
}

/// A move.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Move {
    pub from: Square,
    pub to: Square,
    /// If the move of a pawn triggers a promotion, the target piece should be specified.
    pub promotion: Option<Piece>,
}

/// The possible actions a player can take.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum PlayerAction {
    /// Move a piece on the board.    
    MakeMove(Player, Move),

    /// Resign the match in favor of the opponent.
    Resign(Player),
}

impl PlayerAction {
    pub fn player(&self) -> &Player {
        use PlayerAction::*;
        match self {
            MakeMove(p, _) | Resign(p) => p,
        }
    }
}

/// The reason why a player action was rejected.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary), proptest(no_params))]
#[error(ignore)]
pub enum InvalidPlayerAction {
    #[display(fmt = "the game has ended in a {}", "_0")]
    GameHasEnded(Outcome),

    #[display(fmt = "it's not {} player's turn", "_0.color.to_str()")]
    TurnOfTheOpponent(Player),

    #[display(
        fmt = "the {} player is not allowed move the {} {} from {} to {} with {} promotion",
        "_0.color.to_str()",
        "_1.color.to_str()",
        "_1.piece.to_str()",
        "_2.from",
        "_2.to",
        "_2.promotion.map(|p| p.to_str()).unwrap_or(\"no\")"
    )]
    IllegalMove(Player, Figure, Move),

    #[display(
        fmt = "the {} player attempted to move a nonexistent piece from {} to {}",
        "_0.color.to_str()",
        "_1.from",
        "_1.to"
    )]
    InvalidMove(Player, Move),
}
