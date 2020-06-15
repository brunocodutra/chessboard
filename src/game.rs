use crate::{foreign, Figure, InvalidPlayerAction, Outcome, Player, PlayerAction, Position};
use derivative::Derivative;

#[cfg(test)]
use foreign::MockGame as Rules;

#[cfg(not(test))]
use foreign::Game as Rules;

/// Standard chess rules.
#[derive(Derivative)]
#[derivative(Default(new = "true"))]
pub struct Game {
    #[derivative(Default(value = "Rules::new()"))]
    rules: Rules,
}

impl Game {
    /// Executes a player action if valid, otherwise returns the reason why not.
    pub fn execute(&mut self, action: PlayerAction) -> Result<(), InvalidPlayerAction> {
        use InvalidPlayerAction::*;

        if let Some(result) = self.outcome() {
            return Err(GameHasEnded(result));
        }

        use PlayerAction::*;
        match action {
            MakeMove(m) => {
                if !self.rules.make_move(m.into()) {
                    let square = m.from.into();
                    let board = self.rules.current_position();

                    debug_assert!(!board.legal(m.into()));

                    if let Some(piece) = board.piece_on(square).map(Into::into) {
                        if let Some(color) = board.color_on(square).map(Into::into) {
                            return Err(IllegalMove(self.player(), Figure::new(color, piece), m));
                        }
                    }

                    return Err(InvalidMove(self.player(), m));
                }
            }

            Resign => {
                assert!(self.rules.resign(self.rules.side_to_move()));

                #[cfg(not(test))]
                debug_assert_eq!(self.outcome(), Some(Outcome::Resignation(self.player())));
            }
        }

        if self.rules.can_declare_draw() {
            assert!(self.rules.declare_draw());

            #[cfg(not(test))]
            debug_assert_eq!(self.outcome(), Some(Outcome::Draw));
        }

        Ok(())
    }

    // The result of the game if it has ended.
    pub fn outcome(&self) -> Option<Outcome> {
        self.rules.result().map(Into::into)
    }

    /// The current position on the board.
    pub fn position(&self) -> Position {
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
    use crate::Move;
    use mockall::predicate::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn any_player_action_after_the_game_has_ended_is_invalid(a: PlayerAction, o: Outcome) {
            let mut rules = Rules::new();
            rules.expect_result().times(1).return_const(Some(o.into()));
            assert_eq!(Game { rules }.execute(a), Err(InvalidPlayerAction::GameHasEnded(o)));
        }

        #[test]
        fn players_can_only_play_legal_moves(p: Player, m: Move, f: Figure) {
            let mut board = foreign::MockBoard::new();

            #[cfg(debug_assertions)]
            board.expect_legal()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .return_const(false);

            board.expect_piece_on()
                .with(eq(Into::<foreign::Square>::into(m.from)))
                .return_const(Some(f.piece().into()));

            board.expect_color_on()
                .with(eq(Into::<foreign::Square>::into(m.from)))
                .return_const(Some(f.color().into()));

            let mut rules = Rules::new();

            rules.expect_result().return_const(None);
            rules.expect_side_to_move().return_const(p.color);
            rules.expect_current_position().return_once(move || board);

            rules.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            use PlayerAction::*;
            use InvalidPlayerAction::*;
            assert_eq!(Game { rules }.execute(MakeMove(m)), Err(IllegalMove(p, f, m)));
        }

        #[test]
        fn players_can_only_play_valid_moves(p: Player, m: Move) {
            let mut board = foreign::MockBoard::new();

            #[cfg(debug_assertions)]
            board.expect_legal()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .return_const(false);

            board.expect_piece_on().times(0..=1).return_const(None);
            board.expect_color_on().times(0..=1).return_const(None);

            let mut rules = Rules::new();

            rules.expect_result().return_const(None);
            rules.expect_side_to_move().return_const(p.color);
            rules.expect_current_position().return_once(move || board);

            rules.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(false);

            use PlayerAction::*;
            use InvalidPlayerAction::*;
            assert_eq!(Game { rules }.execute(MakeMove(m)), Err(InvalidMove(p, m)));
        }

        #[test]
        fn players_can_make_valid_and_legal_moves(p: Player, m: Move) {
            let mut rules = Rules::new();

            rules.expect_result().return_const(None);

            rules.expect_make_move()
                .with(eq(Into::<foreign::ChessMove>::into(m)))
                .times(1)
                .return_const(true);

            rules.expect_can_declare_draw().times(1).return_const(false);

            use PlayerAction::*;
            assert_eq!(Game { rules }.execute(MakeMove(m)), Ok(()));
        }

        #[test]
        fn players_can_resign(p: Player) {
            let mut rules = Rules::new();

            rules.expect_result().return_const(None);
            rules.expect_side_to_move().return_const(p.color);

            rules.expect_resign()
                .with(eq(Into::<foreign::Color>::into(p.color)))
                .times(1)
                .return_const(true);

            rules.expect_can_declare_draw().times(1).return_const(false);

            use PlayerAction::*;
            assert_eq!(Game { rules }.execute(Resign), Ok(()));
        }


        #[test]
        fn draw_is_declared_if_the_criteria_is_met(p: Player, a: PlayerAction) {
            let mut rules = Rules::new();

            rules.expect_result().return_const(None);
            rules.expect_side_to_move().return_const(p.color);
            rules.expect_make_move().times(0..=1).return_const(true);
            rules.expect_resign().times(0..=1).return_const(true);

            rules.expect_can_declare_draw().times(1).return_const(true);
            rules.expect_declare_draw().times(1).return_const(true);

            assert_eq!(Game { rules }.execute(a), Ok(()));
        }

        #[test]
        fn outcome_returns_the_result_of_the_game_if_it_has_ended(o: Option<Outcome>) {
            let mut rules = Rules::new();
            rules.expect_result().times(1).return_const(o.map(Into::into));
            assert_eq!(Game { rules }.outcome(), o);
        }

        #[test]
        fn position_returns_the_current_board(p: Position) {
            let mut board = foreign::MockBoard::new();
            board.expect_into::<Position>().times(1).return_const(p);

            let mut rules = Rules::new();
            rules.expect_current_position().times(1).return_once(move || board);

            assert_eq!(Game { rules }.position(), p);
        }

        #[test]
        fn player_returns_the_current_side_to_move(p: Player) {
            let mut rules = Rules::new();
            rules.expect_side_to_move().times(1).return_const(p.color);
            assert_eq!(Game { rules }.player(), p);
        }
    }
}
