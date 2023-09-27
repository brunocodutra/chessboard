use crate::chess::{Role, Square};
use crate::util::{Binary, Bits};
use derive_more::{DebugCustom, Display, Error};
use shakmaty as sm;
use vampirc_uci::UciMove;

#[cfg(test)]
use crate::chess::Position;

#[cfg(test)]
use proptest::{prelude::*, sample::*};

/// A chess move.
#[derive(DebugCustom, Copy, Clone, Eq, PartialEq, Hash)]
#[debug(
    fmt = "Move({whence}, {whither}, {}, {:?}, {:?})",
    "self.role()",
    "self.promotion()",
    "self.capture()"
)]
#[repr(align(4))]
pub struct Move {
    whence: Square,
    whither: Square,
    capture: Bits<u8, 3>,
    role: Bits<u8, 4>,
}

#[cfg(test)]
impl Arbitrary for Move {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        (any::<Position>(), any::<Selector>())
            .prop_filter_map("end position", |(pos, selector)| {
                selector.try_select(pos.moves())
            })
            .boxed()
    }
}

impl Move {
    /// The source [`Square`].
    pub fn whence(&self) -> Square {
        self.whence
    }

    /// The destination [`Square`].
    pub fn whither(&self) -> Square {
        self.whither
    }

    /// Whether this is a promotion move.
    pub fn is_promotion(&self) -> bool {
        self.role.slice(..1).get() == 1
    }

    /// Whether this is a castling move.
    pub fn is_castling(&self) -> bool {
        self.role() == Role::King && (self.whence().file() - self.whither().file()).abs() > 1
    }

    /// Whether this is an en passant capture move.
    pub fn is_en_passant(&self) -> bool {
        self.capture.get() == 0b110
    }

    /// Whether this is a capture move.
    pub fn is_capture(&self) -> bool {
        self.capture.get() != 0b111
    }

    /// Whether this move is neither a capture nor a promotion.
    pub fn is_quiet(&self) -> bool {
        !(self.is_capture() || self.is_promotion())
    }

    /// The [`Role`] of the piece moved.
    pub fn role(&self) -> Role {
        let mut bits = self.role;
        match bits.pop::<u8, 1>().get() {
            0 => Role::decode(bits.pop()).expect("expected valid encoding"),
            _ => Role::Pawn,
        }
    }

    /// The [`Promotion`] specifier.
    pub fn promotion(&self) -> Option<Role> {
        let mut bits = self.role;
        match bits.pop::<u8, 1>().get() {
            0 => None,
            _ => Some(Role::decode(bits.pop()).expect("expected valid encoding")),
        }
    }

    /// The [`Role`] of the piece captured.
    pub fn capture(&self) -> Option<(Role, Square)> {
        if !self.is_capture() {
            None
        } else if self.is_en_passant() {
            Some((
                Role::Pawn,
                Square::new(self.whither.file(), self.whence.rank()),
            ))
        } else {
            Some((
                Role::decode(self.capture).expect("expected valid encoding"),
                self.whither,
            ))
        }
    }
}

/// The reason why decoding [`Move`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[display(fmt = "not a valid move")]
pub struct DecodeMoveError;

impl From<<Square as Binary>::Error> for DecodeMoveError {
    fn from(_: <Square as Binary>::Error) -> Self {
        DecodeMoveError
    }
}

impl From<<Role as Binary>::Error> for DecodeMoveError {
    fn from(_: <Role as Binary>::Error) -> Self {
        DecodeMoveError
    }
}

impl Binary for Move {
    type Bits = Bits<u32, 19>;
    type Error = DecodeMoveError;

    fn encode(&self) -> Self::Bits {
        let mut bits = Bits::default();
        bits.push(self.role);
        bits.push(self.capture);
        bits.push(self.whither.encode());
        bits.push(self.whence.encode());
        bits
    }

    fn decode(mut bits: Self::Bits) -> Result<Self, Self::Error> {
        let whence = Square::decode(bits.pop())?;
        let whither = Square::decode(bits.pop())?;
        let capture = bits.pop::<u8, 3>();
        let role = bits.pop::<u8, 4>();

        // validate encoding
        Role::decode(role.slice(1..).pop())?;

        Ok(Move {
            whence,
            whither,
            capture,
            role,
        })
    }
}

#[doc(hidden)]
impl From<sm::Move> for Move {
    fn from(m: sm::Move) -> Self {
        match m {
            sm::Move::Normal {
                role,
                from,
                capture,
                to,
                promotion,
            } => {
                let mut role = Bits::new(Role::from(promotion.unwrap_or(role)).encode().get());

                match promotion {
                    None => role.push(Bits::<u8, 1>::new(0)),
                    Some(_) => role.push(Bits::<u8, 1>::new(1)),
                }

                Move {
                    whence: from.into(),
                    whither: to.into(),
                    capture: capture.map_or(!Bits::default(), |r| Role::from(r).encode()),
                    role,
                }
            }

            sm::Move::EnPassant { from, to } => {
                let mut role = Bits::new(Role::Pawn.encode().get());
                role.push(Bits::<u8, 1>::new(0));

                Move {
                    whence: from.into(),
                    whither: to.into(),
                    capture: Bits::new(0b110),
                    role,
                }
            }

            sm::Move::Castle { king, rook } => {
                let mut role = Bits::new(Role::King.encode().get());
                role.push(Bits::<u8, 1>::new(0));

                let whither = if rook > king {
                    Square::new(sm::File::G.into(), king.rank().into())
                } else {
                    Square::new(sm::File::C.into(), king.rank().into())
                };

                Move {
                    whence: king.into(),
                    whither,
                    capture: Bits::new(0b111),
                    role,
                }
            }

            v => panic!("unexpected {v:?}"),
        }
    }
}

#[doc(hidden)]
impl From<Move> for sm::Move {
    fn from(m: Move) -> Self {
        if m.is_castling() {
            sm::Move::Castle {
                king: m.whence().into(),
                rook: if m.whence() < m.whither() {
                    sm::Square::from_coords(sm::File::H, m.whither().rank().into())
                } else {
                    sm::Square::from_coords(sm::File::A, m.whither().rank().into())
                },
            }
        } else if m.is_en_passant() {
            sm::Move::EnPassant {
                from: m.whence().into(),
                to: m.whither().into(),
            }
        } else {
            sm::Move::Normal {
                role: m.role().into(),
                from: m.whence().into(),
                capture: m.capture().map(|(r, _)| r.into()),
                to: m.whither().into(),
                promotion: m.promotion().map(Role::into),
            }
        }
    }
}

#[doc(hidden)]
impl From<Move> for UciMove {
    fn from(m: Move) -> Self {
        UciMove {
            from: m.whence().into(),
            to: m.whither().into(),
            promotion: m.promotion().map(Role::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;
    use test_strategy::proptest;

    #[proptest]
    fn move_guarantees_zero_value_optimization() {
        assert_eq!(size_of::<Option<Move>>(), size_of::<Move>());
    }

    #[proptest]
    fn decoding_encoded_move_is_an_identity(m: Move) {
        assert_eq!(Move::decode(m.encode()), Ok(m));
    }

    #[proptest]
    fn move_has_an_equivalent_shakmaty_representation(m: Move) {
        assert_eq!(Move::from(<sm::Move as From<_>>::from(m)), m);
    }

    #[proptest]
    fn castling_moves_are_never_captures(m: Move) {
        assert!(!m.is_castling() || !m.is_capture());
    }

    #[proptest]
    fn castling_moves_are_never_promotions(m: Move) {
        assert!(!m.is_castling() || !m.is_promotion());
    }

    #[proptest]
    fn en_passant_moves_are_always_captures(m: Move) {
        assert!(!m.is_en_passant() || m.is_capture());
    }

    #[proptest]
    fn captures_are_never_quiet(m: Move) {
        assert!(!m.is_capture() || !m.is_quiet());
    }

    #[proptest]
    fn promotions_are_never_quiet(m: Move) {
        assert!(!m.is_promotion() || !m.is_quiet());
    }
}
