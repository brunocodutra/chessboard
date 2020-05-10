use std::fmt;
/// Denotes the color of a chess [Piece].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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

impl fmt::Display for Figure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

/// Denotes a column of the chessboard.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Square {
    pub file: File,
    pub rank: Rank,
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.file.to_str(), self.rank.to_str())
    }
}
