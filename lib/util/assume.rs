/// A trait for types that can be assumed to be another type.
pub trait Assume {
    /// The type of the assumed value.
    type Assumed;

    /// Assume `Self` represents a value of `Self::Assumed`.
    fn assume(self) -> Self::Assumed;
}

impl<T> Assume for Option<T> {
    type Assumed = T;

    fn assume(self) -> Self::Assumed {
        // Definitely not safe, but we'll do it anyway.
        unsafe { self.unwrap_unchecked() }
    }
}

impl<T, E> Assume for Result<T, E> {
    type Assumed = T;

    fn assume(self) -> Self::Assumed {
        // Definitely not safe, but we'll do it anyway.
        unsafe { self.unwrap_unchecked() }
    }
}
