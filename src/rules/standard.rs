use super::ChessRules;
use crate::{action::*, chess::*};
use derivative::Derivative;

#[cfg(not(test))]
use chess as foreign;

#[cfg(test)]
mod foreign {
    pub use chess::*;
    use mockall::mock;

    mock! {
        pub Board {
            fn legal(&self, m: ChessMove) -> bool;
            fn piece_on(&self, square: Square) -> Option<Piece>;
            fn color_on(&self, square: Square) -> Option<Color>;
        }
    }

    pub use MockBoard as Board;

    mock! {
        pub Game {
            fn current_position(&self) -> Board;
            fn make_move(&mut self, m: ChessMove) -> bool;
            fn resign(&mut self, color: Color) -> bool;
            fn result(&self) -> Option<GameResult>;
            fn side_to_move(&self) -> Color;
        }
    }

    pub use MockGame as Game;
}

/// Standard chess rules.
#[derive(Derivative)]
#[derivative(Default)]
pub struct Standard(#[derivative(Default(value = "foreign::Game::new()"))] foreign::Game);

impl From<foreign::Color> for Color {
    fn from(c: foreign::Color) -> Self {
        use Color::*;
        match c {
            foreign::Color::White => White,
            foreign::Color::Black => Black,
        }
    }
}

impl Into<foreign::Color> for Color {
    fn into(self) -> foreign::Color {
        use Color::*;
        match self {
            White => foreign::Color::White,
            Black => foreign::Color::Black,
        }
    }
}

impl From<foreign::Piece> for Piece {
    fn from(p: foreign::Piece) -> Self {
        use Piece::*;
        match p {
            foreign::Piece::Pawn => Pawn,
            foreign::Piece::Knight => Knight,
            foreign::Piece::Bishop => Bishop,
            foreign::Piece::Rook => Rook,
            foreign::Piece::Queen => Queen,
            foreign::Piece::King => King,
        }
    }
}

impl Into<foreign::Piece> for Piece {
    fn into(self: Self) -> foreign::Piece {
        use Piece::*;
        match self {
            Pawn => foreign::Piece::Pawn,
            Knight => foreign::Piece::Knight,
            Bishop => foreign::Piece::Bishop,
            Rook => foreign::Piece::Rook,
            Queen => foreign::Piece::Queen,
            King => foreign::Piece::King,
        }
    }
}

impl Into<foreign::Square> for Square {
    fn into(self: Self) -> foreign::Square {
        let Square { file, rank } = self;

        foreign::Square::make_square(
            foreign::Rank::from_index(rank as usize),
            foreign::File::from_index(file as usize),
        )
    }
}

impl Into<foreign::ChessMove> for Move {
    fn into(self: Self) -> foreign::ChessMove {
        foreign::ChessMove::new(
            self.from.into(),
            self.to.into(),
            self.promotion.map(Piece::into),
        )
    }
}

impl From<foreign::Color> for Player {
    fn from(c: foreign::Color) -> Self {
        Player { color: c.into() }
    }
}

impl From<foreign::GameResult> for Outcome {
    fn from(c: foreign::GameResult) -> Self {
        use Color::*;
        use Outcome::*;
        match c {
            foreign::GameResult::WhiteResigns => Resignation(Player { color: White }),
            foreign::GameResult::BlackResigns => Resignation(Player { color: Black }),
            foreign::GameResult::WhiteCheckmates => Checkmate(Player { color: White }),
            foreign::GameResult::BlackCheckmates => Checkmate(Player { color: Black }),
            foreign::GameResult::Stalemate => Stalemate,
            foreign::GameResult::DrawAccepted | foreign::GameResult::DrawDeclared => Draw,
        }
    }
}

#[cfg(test)]
impl Into<foreign::GameResult> for Outcome {
    fn into(self) -> foreign::GameResult {
        use Color::*;
        use Outcome::*;
        match self {
            Resignation(Player { color: White }) => foreign::GameResult::WhiteResigns,
            Resignation(Player { color: Black }) => foreign::GameResult::BlackResigns,
            Checkmate(Player { color: White }) => foreign::GameResult::WhiteCheckmates,
            Checkmate(Player { color: Black }) => foreign::GameResult::BlackCheckmates,
            Stalemate => foreign::GameResult::Stalemate,
            Draw => foreign::GameResult::DrawDeclared,
        }
    }
}

impl ChessRules for Standard {
    fn execute(&mut self, action: PlayerAction) -> Result<(), InvalidPlayerAction> {
        use InvalidPlayerAction::*;

        if let Some(result) = self.0.result().map(Into::into) {
            return Err(GameHasEnded(result));
        }

        if action.player() != &self.0.side_to_move().into() {
            return Err(TurnOfTheOpponent(*action.player()));
        }

        use PlayerAction::*;
        match action {
            MakeMove(p, m) => {
                if !self.0.make_move(m.into()) {
                    let square = m.from.into();
                    let board = self.0.current_position();

                    debug_assert!(!board.legal(m.into()));

                    if let Some(piece) = board.piece_on(square).map(Into::into) {
                        if let Some(color) = board.color_on(square).map(Into::into) {
                            return Err(IllegalMove(p, Figure { piece, color }, m));
                        }
                    }

                    return Err(InvalidMove(p, m));
                }
            }

            Resign(p) => assert!(self.0.resign(p.color.into())),
        }

        Ok(())
    }

    fn outcome(&self) -> Option<Outcome> {
        self.0.result().map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_player_action_after_the_game_has_ended_is_invalid(a: PlayerAction, o: Outcome) {
            let mut game = foreign::Game::new();
            game.expect_result().times(1).return_const(Some(o.into()));
            assert_eq!(Standard(game).execute(a), Err(InvalidPlayerAction::GameHasEnded(o)));
        }

        #[test]
        fn players_can_only_act_in_their_turn(a: PlayerAction) {
            let mut game = foreign::Game::new();

            game.expect_result().times(1).return_const(None);
            game.expect_side_to_move().times(1).returning(move || match a.player() {
                Player { color: Color::White } => foreign::Color::Black,
                Player { color: Color::Black } => foreign::Color::White,
            });

            use InvalidPlayerAction::*;
            assert_eq!(Standard(game).execute(a), Err(TurnOfTheOpponent(*a.player())));
        }

        #[test]
        fn players_can_only_play_legal_moves(p: Player, m: Move, f: Figure) {
            let mut board = foreign::Board::new();

            #[cfg(debug_assertions)]
            board.expect_legal()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            board.expect_piece_on()
                .with(eq(Into::<foreign::Square>::into(m.from)))
                .times(1)
                .return_const(Some(f.piece.into()));

            board.expect_color_on()
                .with(eq(Into::<foreign::Square>::into(m.from)))
                .times(1)
                .return_const(Some(f.color.into()));

            let mut game = foreign::Game::new();

            game.expect_result().times(1).return_const(None);
            game.expect_side_to_move().times(1).return_const(p.color);
            game.expect_current_position().times(1).return_once(move || board);

            game.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            use PlayerAction::*;
            use InvalidPlayerAction::*;
            assert_eq!(Standard(game).execute(MakeMove(p, m)), Err(IllegalMove(p, f, m)));
        }

        #[test]
        fn players_can_only_play_valid_moves(p: Player, m: Move) {
            let mut board = foreign::Board::new();

            #[cfg(debug_assertions)]
            board.expect_legal()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            board.expect_piece_on().times(0..=1).return_const(None);
            board.expect_color_on().times(0..=1).return_const(None);

            let mut game = foreign::Game::new();

            game.expect_result().times(1).return_const(None);
            game.expect_side_to_move().times(1).return_const(p.color);
            game.expect_current_position().times(1).return_once(move || board);

            game.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            use PlayerAction::*;
            use InvalidPlayerAction::*;
            assert_eq!(Standard(game).execute(MakeMove(p, m)), Err(InvalidMove(p, m)));
        }

        #[test]
        fn players_can_make_valid_and_legal_moves(p: Player, m: Move) {
            let mut game = foreign::Game::new();

            game.expect_result().times(1).return_const(None);
            game.expect_side_to_move().times(1).return_const(p.color);

            game.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(true);

            use PlayerAction::*;
            assert_eq!(Standard(game).execute(MakeMove(p, m)), Ok(()));
        }

        #[test]
        fn players_can_resign(p: Player) {
            let mut game = foreign::Game::new();

            game.expect_result().times(1).return_const(None);
            game.expect_side_to_move().times(1).return_const(p.color);

            game.expect_resign()
                .with(eq(Into::<foreign::Color>::into(p.color)))
                .times(1)
                .return_const(true);

            use PlayerAction::*;
            assert_eq!(Standard(game).execute(Resign(p)), Ok(()));
        }

        #[test]
        fn outcome_returns_the_result_of_the_game_if_it_has_ended(o: Option<Outcome>) {
            let mut game = foreign::Game::new();
            game.expect_result().times(1).return_const(o.map(Into::into));
            assert_eq!(Standard(game).outcome(), o);
        }
    }
}
