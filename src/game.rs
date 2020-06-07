use crate::{foreign, Figure, InvalidPlayerAction, Outcome, Placement, Player, PlayerAction};
use derivative::Derivative;

/// Standard chess rules.
#[derive(Derivative)]
#[derivative(Default(new = "true"))]
pub struct Game {
    #[derivative(Default(value = "foreign::Game::new()"))]
    rules: foreign::Game,
}

impl Game {
    /// Executes a player action if valid, otherwise returns the reason why not.
    pub fn execute(&mut self, action: PlayerAction) -> Result<(), InvalidPlayerAction> {
        use InvalidPlayerAction::*;

        if let Some(result) = self.rules.result().map(Into::into) {
            return Err(GameHasEnded(result));
        }

        if action.player().color != self.rules.side_to_move().into() {
            return Err(TurnOfTheOpponent(*action.player()));
        }

        use PlayerAction::*;
        match action {
            MakeMove(p, m) => {
                if !self.rules.make_move(m.into()) {
                    let square = m.from.into();
                    let board = self.rules.current_position();

                    debug_assert!(!board.legal(m.into()));

                    if let Some(piece) = board.piece_on(square).map(Into::into) {
                        if let Some(color) = board.color_on(square).map(Into::into) {
                            return Err(IllegalMove(p, Figure::new(color, piece), m));
                        }
                    }

                    return Err(InvalidMove(p, m));
                }
            }

            Resign(p) => assert!(self.rules.resign(p.color.into())),
        }

        Ok(())
    }

    // The result of the game if it has ended.
    pub fn outcome(&self) -> Option<Outcome> {
        self.rules.result().map(Into::into)
    }

    /// The current position on the board.
    pub fn position(&self) -> Placement {
        self.rules.current_position().into()
    }

    /// The player of the current turn.
    pub fn player(&self) -> Player {
        Player {
            color: self.rules.side_to_move().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Move};
    use mockall::predicate::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_player_action_after_the_game_has_ended_is_invalid(a: PlayerAction, o: Outcome) {
            let mut rules = foreign::Game::new();
            rules.expect_result().times(1).return_const(Some(o.into()));
            assert_eq!(Game { rules }.execute(a), Err(InvalidPlayerAction::GameHasEnded(o)));
        }

        #[test]
        fn players_can_only_act_in_their_turn(a: PlayerAction) {
            let mut rules = foreign::Game::new();

            rules.expect_result().times(1).return_const(None);
            rules.expect_side_to_move().times(1).returning(move || match a.player() {
                Player { color: Color::White } => foreign::Color::Black,
                Player { color: Color::Black } => foreign::Color::White,
            });

            use InvalidPlayerAction::*;
            assert_eq!(Game { rules }.execute(a), Err(TurnOfTheOpponent(*a.player())));
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
                .return_const(Some(f.piece().into()));

            board.expect_color_on()
                .with(eq(Into::<foreign::Square>::into(m.from)))
                .times(1)
                .return_const(Some(f.color().into()));

            let mut rules = foreign::Game::new();

            rules.expect_result().times(1).return_const(None);
            rules.expect_side_to_move().times(1).return_const(p.color);
            rules.expect_current_position().times(1).return_once(move || board);

            rules.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            use PlayerAction::*;
            use InvalidPlayerAction::*;
            assert_eq!(Game { rules }.execute(MakeMove(p, m)), Err(IllegalMove(p, f, m)));
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

            let mut rules = foreign::Game::new();

            rules.expect_result().times(1).return_const(None);
            rules.expect_side_to_move().times(1).return_const(p.color);
            rules.expect_current_position().times(1).return_once(move || board);

            rules.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            use PlayerAction::*;
            use InvalidPlayerAction::*;
            assert_eq!(Game { rules }.execute(MakeMove(p, m)), Err(InvalidMove(p, m)));
        }

        #[test]
        fn players_can_make_valid_and_legal_moves(p: Player, m: Move) {
            let mut rules = foreign::Game::new();

            rules.expect_result().times(1).return_const(None);
            rules.expect_side_to_move().times(1).return_const(p.color);

            rules.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(true);

            use PlayerAction::*;
            assert_eq!(Game { rules }.execute(MakeMove(p, m)), Ok(()));
        }

        #[test]
        fn players_can_resign(p: Player) {
            let mut rules = foreign::Game::new();

            rules.expect_result().times(1).return_const(None);
            rules.expect_side_to_move().times(1).return_const(p.color);

            rules.expect_resign()
                .with(eq(Into::<foreign::Color>::into(p.color)))
                .times(1)
                .return_const(true);

            use PlayerAction::*;
            assert_eq!(Game { rules }.execute(Resign(p)), Ok(()));
        }

        #[test]
        fn outcome_returns_the_result_of_the_game_if_it_has_ended(o: Option<Outcome>) {
            let mut rules = foreign::Game::new();
            rules.expect_result().times(1).return_const(o.map(Into::into));
            assert_eq!(Game { rules }.outcome(), o);
        }

        #[test]
        fn position_returns_the_current_board(p: Placement) {
            let mut board = foreign::Board::new();

            board.expect_piece_on().times(0..=64).returning(move |s| p[s.into()].map(|f| f.piece().into()));
            board.expect_color_on().times(0..=64).returning(move |s| p[s.into()].map(|f| f.color().into()));

            let mut rules = foreign::Game::new();
            rules.expect_current_position().times(1).return_once(move || board);

            assert_eq!(Game { rules }.position(), p);
        }

        #[test]
        fn player_returns_the_current_side_to_move(p: Player) {
            let mut rules = foreign::Game::new();
            rules.expect_side_to_move().times(1).return_const(p.color);
            assert_eq!(Game { rules }.player(), p);
        }
    }
}
