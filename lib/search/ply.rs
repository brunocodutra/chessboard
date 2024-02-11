use crate::util::{Integer, Saturating};

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[repr(transparent)]
pub struct PlyRepr(#[cfg_attr(test, strategy(Self::RANGE))] <Self as Integer>::Repr);

unsafe impl Integer for PlyRepr {
    type Repr = i8;

    const MIN: Self::Repr = -Self::MAX;

    #[cfg(not(test))]
    const MAX: Self::Repr = 127;

    #[cfg(test)]
    const MAX: Self::Repr = 3;

    #[inline(always)]
    fn repr(&self) -> Self::Repr {
        self.0
    }
}

/// The number of half-moves played.
pub type Ply = Saturating<PlyRepr>;
