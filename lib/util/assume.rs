use std::fmt::Debug;

/// A trait for types that can be assumed to be another type.
pub trait Assume {
    /// The type of the assumed value.
    type Assumed;

    /// Assume `Self` represents a value of `Self::Assumed`.
    fn assume(self) -> Self::Assumed;
}

impl<T> Assume for Option<T> {
    type Assumed = T;

    #[inline(always)]
    #[track_caller]
    fn assume(self) -> Self::Assumed {
        #[cfg(not(test))]
        unsafe {
            // Definitely not safe, but we'll assume unit tests will catch everything.
            self.unwrap_unchecked()
        }

        #[cfg(test)]
        self.unwrap()
    }
}

impl<T, E: Debug> Assume for Result<T, E> {
    type Assumed = T;

    #[inline(always)]
    #[track_caller]
    fn assume(self) -> Self::Assumed {
        #[cfg(not(test))]
        unsafe {
            // Definitely not safe, but we'll assume unit tests will catch everything.
            self.unwrap_unchecked()
        }

        #[cfg(test)]
        self.unwrap()
    }
}
