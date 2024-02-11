use crate::util::{Integer, Saturating};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct ValueRepr(#[cfg_attr(test, strategy(Self::RANGE))] <Self as Integer>::Repr);

unsafe impl Integer for ValueRepr {
    type Repr = i16;

    const MIN: Self::Repr = -Self::MAX;
    const MAX: Self::Repr = 8000;

    #[inline(always)]
    fn repr(&self) -> Self::Repr {
        self.0
    }
}

/// A position's static evaluation.
pub type Value = Saturating<ValueRepr>;
