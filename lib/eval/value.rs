use crate::util::{Binary, Bits};
use derive_more::{DebugCustom, Display, Error, Neg};
use num_traits::{clamp, AsPrimitive};
use std::ops::{Add, Sub};
use test_strategy::Arbitrary;

#[derive(
    DebugCustom, Display, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Arbitrary, Neg,
)]
#[debug(fmt = "Value({})", self)]
#[display(fmt = "{:+}", _0)]
pub struct Value(#[strategy(Self::MIN.get()..=Self::MAX.get())] i16);

impl Value {
    pub const ZERO: Self = Value(0);
    pub const MIN: Self = Value(-4095);
    pub const MAX: Self = Value(4095);

    /// Constructs [`Value`] from a raw value.
    ///
    /// # Panics
    ///
    /// Panics if `v` is outside of the bounds.
    #[inline]
    pub fn new(v: i16) -> Self {
        v.try_into().unwrap()
    }

    /// Returns the raw value.
    #[inline]
    pub fn get(&self) -> i16 {
        self.0
    }

    /// Safely constructs [`Value`] from a raw value through saturation.
    #[inline]
    pub fn saturate<T: AsPrimitive<i16> + From<i16> + PartialOrd>(i: T) -> Self {
        Value(clamp(i, Self::MIN.get().into(), Self::MAX.get().into()).as_())
    }
}

/// The reason why converting [`Value`] from an integer failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Error)]
#[display(
    fmt = "expected integer in the range `({}..={})`",
    Value::MIN,
    Value::MAX
)]
pub struct ValueOutOfRange;

impl TryFrom<i16> for Value {
    type Error = ValueOutOfRange;

    #[inline]
    fn try_from(v: i16) -> Result<Self, Self::Error> {
        if (Value::MIN.get()..=Value::MAX.get()).contains(&v) {
            Ok(Value(v))
        } else {
            Err(ValueOutOfRange)
        }
    }
}

impl Add<i16> for Value {
    type Output = Self;

    #[inline]
    fn add(self, rhs: i16) -> Self::Output {
        Value::saturate(self.get().saturating_add(rhs))
    }
}

impl Add for Value {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        self + rhs.get()
    }
}

impl Sub<i16> for Value {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: i16) -> Self::Output {
        Value::saturate(self.get().saturating_sub(rhs))
    }
}

impl Sub for Value {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        self - rhs.get()
    }
}

/// The reason why decoding [`Value`] from binary failed.
#[derive(Debug, Display, Clone, Eq, PartialEq, Arbitrary, Error)]
#[display(fmt = "not a valid Value")]
pub struct DecodeValueError;

impl Binary for Value {
    type Bits = Bits<u16, 13>;
    type Error = DecodeValueError;

    fn encode(&self) -> Self::Bits {
        Bits::new((self.0 - Self::MIN.get()) as _)
    }

    fn decode(bits: Self::Bits) -> Result<Self, Self::Error> {
        if bits == !Bits::default() {
            Err(DecodeValueError)
        } else {
            Ok(Value::new(bits.get() as i16 + Self::MIN.get()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn new_accepts_values_within_bounds(#[strategy(Value::MIN.get()..=Value::MAX.get())] v: i16) {
        assert_eq!(Value::new(v), Value(v));
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_value_greater_than_max(#[strategy(Value::MAX.get() + 1..)] v: i16) {
        Value::new(v);
    }

    #[proptest]
    #[should_panic]
    fn new_panics_if_value_smaller_than_min(#[strategy(..Value::MIN.get())] v: i16) {
        Value::new(v);
    }

    #[proptest]
    fn saturate_preserves_values_within_bounds(
        #[strategy(Value::MIN.get()..=Value::MAX.get())] v: i16,
    ) {
        assert_eq!(Value::saturate(v), Value(v));
    }

    #[proptest]
    fn saturate_caps_if_value_greater_than_max(#[strategy(Value::MAX.get() + 1..)] v: i16) {
        assert_eq!(Value::saturate(v), Value::MAX);
    }

    #[proptest]
    fn saturate_caps_if_value_smaller_than_min(#[strategy(..Value::MIN.get())] v: i16) {
        assert_eq!(Value::saturate(v), Value::MIN);
    }

    #[proptest]
    fn get_returns_raw_value(v: Value) {
        assert_eq!(v.get(), v.0);
    }

    #[proptest]
    fn double_negation_is_idempotent(v: Value) {
        assert_eq!(-(-v), v);
    }

    #[proptest]
    fn addition_is_symmetric(a: Value, b: Value) {
        assert_eq!(a + b, b + a);
    }

    #[proptest]
    fn subtraction_is_antisymmetric(a: Value, b: Value) {
        assert_eq!(a - b, -(b - a));
    }

    #[proptest]
    fn decoding_encoded_value_is_an_identity(v: Value) {
        assert_eq!(Binary::decode(v.encode()), Ok(v));
    }

    #[proptest]
    fn decoding_value_fails_for_invalid_bits() {
        let b = !<Value as Binary>::Bits::default();
        assert_eq!(Value::decode(b), Err(DecodeValueError));
    }
}
