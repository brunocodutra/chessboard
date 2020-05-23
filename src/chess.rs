use derive_more::Display;

/// Denotes the color of a chess [Piece].
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

/// Denotes a chess piece.
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

/// Denotes a chess piece of a certain color.
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

/// Denotes a column of the chessboard.
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

/// Denotes a row of the chessboard.
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

/// Denotes a square of the chessboard.
#[derive(Debug, Display, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
#[display(fmt = "{}{}", "self.file.to_str()", "self.rank.to_str()")]
pub struct Square {
    pub file: File,
    pub rank: Rank,
}

/// Denotes a player by color.
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
