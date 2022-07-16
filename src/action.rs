use crate::{Binary, Bits, IllegalMove, Move, Outcome};
use derive_more::{DebugCustom, Display, Error, From};

/// The possible actions a player can take.
#[derive(DebugCustom, Display, Copy, Clone, Eq, PartialEq, Hash, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub enum Action {
    /// Move a piece on the board.
    #[debug(fmt = "{:?}", _0)]
    #[display(fmt = "{}", _0)]
    Move(Move),

    /// Resign the game in favor of the opponent.
    #[display(fmt = "resign")]
    #[from(ignore)]
    Resign,
}

/// The reason why the player [`Action`] was rejected.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error, From)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[error(ignore)]
pub enum IllegalAction {
    #[display(fmt = "the game has already ended in a {}", _0)]
    GameHasEnded(Outcome),

    #[display(fmt = "{}", _0)]
    PlayerAttemptedIllegalMove(IllegalMove),
}

/// The reason why decoding [`Action`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Hash, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "`{}` is not a valid Action", _0)]
pub struct DecodeActionError(#[error(not(source))] <Action as Binary>::Register);

impl Binary for Action {
    type Register = Bits<u16, 15>;
    type Error = DecodeActionError;

    fn encode(&self) -> Self::Register {
        match self {
            Action::Resign => Bits::max(),
            Action::Move(m) => {
                let register = m.encode();
                debug_assert_ne!(register, Bits::max());
                register
            }
        }
    }

    fn decode(register: Self::Register) -> Result<Self, Self::Error> {
        if register == Bits::max() {
            Ok(Action::Resign)
        } else {
            Ok(Action::Move(
                Move::decode(register).map_err(|_| DecodeActionError(register))?,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn action_can_be_converted_from_move(m: Move) {
        assert_eq!(Action::from(m), Action::Move(m));
    }

    #[proptest]
    fn decoding_encoded_action_is_an_identity(a: Action) {
        assert_eq!(Action::decode(a.encode()), Ok(a));
    }

    #[proptest]
    fn decoding_action_fails_for_invalid_register(
        #[filter(#b != Bits::max())]
        #[any(64 * 64 * 5)]
        b: Bits<u16, 15>,
    ) {
        assert_eq!(Action::decode(b), Err(DecodeActionError(b)));
    }

    #[proptest]
    fn illegal_action_can_be_converted_from_outcome(o: Outcome) {
        assert_eq!(IllegalAction::from(o), IllegalAction::GameHasEnded(o));
    }

    #[proptest]
    fn illegal_action_can_be_converted_from_illegal_move(im: IllegalMove) {
        assert_eq!(
            IllegalAction::from(im.clone()),
            IllegalAction::PlayerAttemptedIllegalMove(im)
        );
    }
}
