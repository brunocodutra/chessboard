pub use chess::*;

#[cfg(test)]
mockall::mock! {
    pub Board {
        fn legal(&self, m: ChessMove) -> bool;
        fn piece_on(&self, square: Square) -> Option<Piece>;
        fn color_on(&self, square: Square) -> Option<Color>;
    }
}

#[cfg(test)]
pub use MockBoard as Board;

#[cfg(test)]
mockall::mock! {
    pub Game {
        fn current_position(&self) -> Board;
        fn make_move(&mut self, m: ChessMove) -> bool;
        fn resign(&mut self, color: Color) -> bool;
        fn result(&self) -> Option<GameResult>;
        fn side_to_move(&self) -> Color;
    }
}

#[cfg(test)]
pub use MockGame as Game;
