#![allow(clippy::unused_unit)]

pub use chess::*;

#[cfg(test)]
mockall::mock! {
    pub Board {
        pub fn legal(&self, m: ChessMove) -> bool;
        pub fn piece_on(&self, square: Square) -> Option<Piece>;
        pub fn color_on(&self, square: Square) -> Option<Color>;
        pub fn into<T: 'static>(self) -> T;
    }
}

#[cfg(test)]
mockall::mock! {
    pub Game {
        pub fn current_position(&self) -> MockBoard;
        pub fn make_move(&mut self, m: ChessMove) -> bool;
        pub fn resign(&mut self, color: Color) -> bool;
        pub fn result(&self) -> Option<GameResult>;
        pub fn side_to_move(&self) -> Color;
        pub fn can_declare_draw(&self) -> bool;
        pub fn declare_draw(&self) -> bool;
    }
}
