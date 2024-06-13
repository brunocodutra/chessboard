use crate::chess::Perspective;
use crate::util::{Integer, Saturating};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct ValueRepr(#[cfg_attr(test, strategy(Self::MIN..=Self::MAX))] <Self as Integer>::Repr);

unsafe impl const Integer for ValueRepr {
    type Repr = i16;
    const MIN: Self::Repr = -Self::MAX;
    const MAX: Self::Repr = 8000;
}

/// A position's static evaluation.
pub type Value = Saturating<ValueRepr>;

impl const Perspective for Value {
    #[inline(always)]
    fn flip(&self) -> Self {
        -*self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[proptest]
    fn flipping_value_produces_its_negative(v: Value) {
        assert_eq!(v.flip(), -v);
    }
}
