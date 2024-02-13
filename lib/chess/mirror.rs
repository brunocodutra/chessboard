use crate::util::Integer;

/// Trait for types that can be mirrored.
pub trait Mirror {
    /// This value's mirror.
    fn mirror(&self) -> Self;
}

impl<T: Integer> Mirror for T {
    /// This value's mirror.
    #[inline(always)]
    fn mirror(&self) -> Self {
        Self::from_repr(Self::MAX - self.repr() + Self::MIN)
    }
}
