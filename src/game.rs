use crate::{Act, Action, Color, GameReport, IllegalAction, Outcome, Position, San};
use anyhow::Context;
use derive_more::{Display, Error};
use shakmaty as sm;
use std::iter::once;
use tracing::{info, instrument, warn};

/// The reason why the [`Game`] was interrupted.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "the {} player encountered an error")]
pub enum GameInterrupted<W, B> {
    #[display(fmt = "white")]
    White(W),

    #[display(fmt = "black")]
    Black(B),
}

/// Holds the state of a game of chess.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = Option<Outcome>))]
pub struct Game {
    position: Position,
    #[cfg_attr(test, strategy(proptest::strategy::Just(*args)))]
    outcome: Option<Outcome>,
}

impl Game {
    /// The current [`Position`].
    pub fn position(&self) -> &Position {
        &self.position
    }

    /// The [`Outcome`] of the game if it has already ended.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// An iterator over the legal [`Action`]s that can be played in this position.
    pub fn actions(&self) -> impl Iterator<Item = Action> {
        let moves = self.position().moves().map(Action::Move);

        let take = if self.outcome().is_none() {
            moves.len() + 1
        } else {
            0
        };

        moves.chain(once(Action::Resign)).take(take)
    }

    /// Executes an [`Action`] if legal, otherwise returns the reason why not.
    ///
    /// If the action is legal, a [`San`] recording the move is returned.
    pub fn execute(&mut self, action: Action) -> Result<San, IllegalAction> {
        if let Some(result) = self.outcome() {
            return Err(IllegalAction::GameHasEnded(result));
        }

        match action {
            Action::Move(m) => {
                let san = self.position.play(m)?;

                self.outcome = if sm::Position::is_insufficient_material(self.position.as_ref()) {
                    Some(Outcome::DrawByInsufficientMaterial)
                } else if self.position.moves().len() > 0 {
                    None
                } else if self.position.checkers().len() > 0 {
                    Some(Outcome::Checkmate(!self.position.turn()))
                } else {
                    Some(Outcome::Stalemate)
                };

                Ok(san)
            }

            Action::Resign => {
                self.outcome = Some(Outcome::Resignation(self.position.turn()));

                let noop = sm::Move::Put {
                    role: sm::Role::King,
                    to: sm::Position::our(self.position.as_ref(), sm::Role::King)
                        .first()
                        .expect("expected king on the board"),
                };

                sm::Position::play_unchecked(self.position.as_mut(), &noop);

                Ok(San::null())
            }
        }
    }

    /// Challenge two players for a game of chess.
    #[instrument(level = "trace", err, ret, skip(white, black))]
    pub async fn run<W: Act, B: Act>(
        &mut self,
        mut white: W,
        mut black: B,
    ) -> Result<GameReport, GameInterrupted<W::Error, B::Error>> {
        let mut moves = Vec::new();

        loop {
            match self.outcome() {
                Some(outcome) => break Ok(GameReport { outcome, moves }),

                None => {
                    let turn = self.position.turn();

                    use GameInterrupted::*;
                    let action = match turn {
                        Color::White => white.act(self).await.map_err(White)?,
                        Color::Black => black.act(self).await.map_err(Black)?,
                    };

                    info!(position = %self.position, player = %turn, %action);

                    match self.execute(action).context("illegal player action") {
                        Err(e) => warn!("{:?}", e),
                        Ok(san) => moves.push(san),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MockAct, Move};
    use proptest::{prop_assume, sample::select};
    use test_strategy::proptest;
    use tokio::runtime;

    #[proptest]
    fn position_borrows_the_current_game_state(g: Game) {
        assert_eq!(g.position(), &g.position);
    }

    #[proptest]
    fn outcome_returns_some_result_if_game_has_ended(o: Outcome, #[any(Some(#o))] g: Game) {
        assert_eq!(g.outcome(), Some(o));
    }

    #[proptest]
    fn all_legal_moves_are_possible_before_game_has_ended(g: Game) {
        assert_eq!(
            g.actions()
                .filter_map(|a| match a {
                    Action::Move(m) => Some(m),
                    Action::Resign => None,
                })
                .collect::<Vec<_>>(),
            g.position().moves().collect::<Vec<_>>()
        );
    }

    #[proptest]
    fn resigning_is_always_possible_before_game_has_ended(g: Game) {
        assert_eq!(g.actions().filter(|a| a == &Action::Resign).count(), 1);
    }

    #[proptest]
    fn there_are_no_further_actions_if_game_has_ended(_o: Outcome, #[any(Some(#_o))] g: Game) {
        assert_eq!(g.actions().count(), 0);
    }

    #[proptest]
    fn legal_actions_can_always_be_executed(
        #[by_ref] mut g: Game,
        #[strategy(select(#g.actions().collect::<Vec<_>>()))] a: Action,
    ) {
        assert_eq!(g.execute(a).err(), None);
    }

    #[proptest]
    fn legal_actions_always_change_the_side_to_play(
        #[by_ref] mut g: Game,
        #[strategy(select(#g.actions().collect::<Vec<_>>()))] a: Action,
    ) {
        let turn = !g.position.turn();
        g.execute(a)?;
        assert_eq!(g.position().turn(), turn);
    }

    #[proptest]
    fn actions_cannot_be_executed_after_game_has_ended(
        o: Outcome,
        #[any(Some(#o))] mut g: Game,
        a: Action,
    ) {
        assert_eq!(g.execute(a), Err(IllegalAction::GameHasEnded(o)));
    }

    #[proptest]
    fn game_state_does_not_change_after_an_invalid_action(
        #[by_ref] mut g: Game,
        #[filter(#g.actions().all(|a| a != #a))] a: Action,
    ) {
        let before = g.clone();
        assert_eq!(g.execute(a).ok(), None);
        assert_eq!(g, before);
    }

    #[proptest]
    fn game_ends_once_an_outcome_is_reached(o: Outcome, #[any(Some(#o))] mut g: Game) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let w = MockAct::new();
        let b = MockAct::new();

        assert_eq!(rt.block_on(g.run(w, b)).ok().map(|r| r.outcome), Some(o));
    }

    #[proptest]
    fn game_returns_sequence_of_moves_in_standard_notation(
        #[by_ref]
        #[filter(#g.position.moves().len() > 0)]
        mut g: Game,
        #[strategy(select(#g.position.moves().collect::<Vec<_>>()))] m: Move,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = g.position.turn();
        let san = g.position.clone().play(m)?;

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let (p, q) = match turn {
            Color::White => (&mut w, &mut b),
            Color::Black => (&mut b, &mut w),
        };

        p.expect_act().return_const(Ok(Action::Move(m)));
        q.expect_act().return_const(Ok(Action::Resign));

        let report = rt.block_on(g.run(w, b));

        prop_assume!(g.outcome() == Some(Outcome::Resignation(!turn)));
        assert_eq!(report.map(|r| r.moves), Ok(vec![san, San::null()]));
    }

    #[proptest]
    fn game_executes_player_actions_in_their_turn(mut g: Game) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = g.position.turn();

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_act().return_const(Ok(Action::Resign));

        assert_eq!(
            rt.block_on(g.run(w, b)).map(|r| r.outcome),
            Ok(Outcome::Resignation(turn))
        );
    }

    #[proptest]
    fn game_ignores_illegal_player_actions(
        #[by_ref] mut g: Game,
        #[filter(#g.actions().all(|a| a != #a))] a: Action,
    ) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = g.position.turn();

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_act()
            .return_const(Ok(a))
            .return_const(Ok(Action::Resign));

        assert_eq!(
            rt.block_on(g.run(w, b)).map(|r| r.outcome),
            Ok(Outcome::Resignation(turn))
        );
    }

    #[proptest]
    fn game_interrupts_if_player_encounters_an_error(mut g: Game, e: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;

        let turn = g.position.turn();

        let mut w = MockAct::new();
        let mut b = MockAct::new();

        let p = match turn {
            Color::White => &mut w,
            Color::Black => &mut b,
        };

        p.expect_act().return_const(Err(e.clone()));

        assert_eq!(
            rt.block_on(g.run(w, b)),
            match turn {
                Color::White => Err(GameInterrupted::White(e)),
                Color::Black => Err(GameInterrupted::Black(e)),
            }
        );
    }
}
