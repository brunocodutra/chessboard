use crate::{Action, Color, InvalidAction, Outcome, Position};
use tracing::instrument;

/// Holds the state of a game of chess.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct Game {
    position: Position,
    resigned: Option<Color>,
}

impl Game {
    /// The current [`Position`].
    pub fn position(&self) -> &Position {
        &self.position
    }

    // The [`Outcome`] of the game if it has already ended.
    pub fn outcome(&self) -> Option<Outcome> {
        if let Some(p) = self.resigned {
            Outcome::Resignation(p).into()
        } else if self.position().is_checkmate() {
            Outcome::Checkmate(!self.position().turn()).into()
        } else if self.position().is_stalemate() {
            Outcome::Stalemate.into()
        } else if self.position().is_draw() {
            Outcome::Draw.into()
        } else {
            None
        }
    }

    /// Executes a player action if valid, otherwise returns the reason why not.
    #[instrument(level = "trace", err)]
    pub fn execute(&mut self, action: Action) -> Result<(), InvalidAction> {
        if let Some(result) = self.outcome() {
            return Err(InvalidAction::GameHasEnded(result));
        }

        match action {
            Action::Move(m) => self.position.play(m)?,

            Action::Resign(p) => {
                self.resigned.replace(p);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Checkmate, Draw, Move, Stalemate};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn position_borrows_the_current_game_state(game: Game) {
            assert_eq!(game.position(), &game.position);
        }

        #[test]
        fn outcome_returns_some_result_if_a_player_has_resigned(pos: Position, p: Color) {
            let game = Game { position: pos, resigned: Some(p) };
            assert_eq!(game.outcome(), Some(Outcome::Resignation(p)));
        }

        #[test]
        fn outcome_returns_some_result_on_a_checkmate_position(pos: Checkmate) {
            let game = Game { position: pos.into(), resigned: None };
            assert_eq!(game.outcome(), Some(Outcome::Checkmate(!game.position().turn())));
        }

        #[test]
        fn outcome_returns_some_result_on_a_stalemate_position(pos: Stalemate) {
            let game = Game { position: pos.into(), resigned: None };
            assert_eq!(game.outcome(), Some(Outcome::Stalemate));
        }

        #[test]
        fn outcome_returns_some_result_on_a_draw_position(pos: Draw) {
            let game = Game { position: pos.into(), resigned: None };
            assert_eq!(game.outcome(), Some(Outcome::Draw));
        }

        #[test]
        fn any_player_action_after_one_side_has_resigned_is_invalid(pos: Position, p: Color, a: Action) {
            let mut game = Game { position: pos, resigned: Some(p) };
            assert_eq!(game.execute(a), Err(game.outcome().unwrap().into()));
        }

        #[test]
        fn any_player_action_on_a_checkmate_position_is_invalid(pos: Checkmate, a: Action) {
            let mut game = Game { position: pos.into(), resigned: None };
            assert_eq!(game.execute(a), Err(game.outcome().unwrap().into()));
        }

        #[test]
        fn any_player_action_on_a_stalemate_position_is_invalid(pos: Stalemate, a: Action) {
            let mut game = Game { position: pos.into(), resigned: None };
            assert_eq!(game.execute(a), Err(game.outcome().unwrap().into()));
        }

        #[test]
        fn any_player_action_on_a_draw_position_is_invalid(pos: Draw, a: Action) {
            let mut game = Game { position: pos.into(), resigned: None };
            assert_eq!(game.execute(a), Err(game.outcome().unwrap().into()));
        }

        #[test]
        fn game_state_does_not_change_after_an_invalid_action(mut game: Game, a: Action) {
            let before = game.clone();
            prop_assume!(game.execute(a).is_err());
            assert_eq!(game, before);
        }

        #[test]
        fn players_can_resign(mut pos: Position, p: Color) {
            prop_assume!(!pos.is_checkmate());
            prop_assume!(!pos.is_stalemate());
            prop_assume!(!pos.is_draw());

            let mut game = Game { position: pos.clone(), resigned: None };
            assert_eq!(game.execute(Action::Resign(p)), Ok(()));
            assert_eq!(game, Game { position: pos, resigned: Some(p) });
        }

        #[test]
        fn players_can_make_a_move(mut pos: Position, m: Move) {
            prop_assume!(!pos.is_checkmate());
            prop_assume!(!pos.is_stalemate());
            prop_assume!(!pos.is_draw());

            let mut game = Game { position: pos.clone(), resigned: None };
            assert_eq!(game.execute(Action::Move(m)), pos.play(m).map_err(Into::into));
            assert_eq!(game, Game { position: pos, resigned: None });
        }
    }
}
