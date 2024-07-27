/// A trait for types that can be assumed to be another type.
pub trait Assume {
    /// The type of the assumed value.
    type Assumed;

    /// Assume `Self` represents a value of `Self::Assumed`.
    fn assume(self) -> Self::Assumed;
}

impl<T> Assume for Option<T> {
    type Assumed = T;

    #[inline]
    #[track_caller]
    fn assume(self) -> Self::Assumed {
        match self {
            Some(v) => v,

            #[cfg(not(debug_assertions))]
            // Definitely not safe, but we'll assume unit tests will catch everything.
            _ => unsafe { std::hint::unreachable_unchecked() },

            #[cfg(debug_assertions)]
            _ => unreachable!(),
        }
    }
}

impl<T, E> Assume for Result<T, E> {
    type Assumed = T;

    #[inline]
    #[track_caller]
    fn assume(self) -> Self::Assumed {
        match self {
            Ok(v) => v,

            #[cfg(not(debug_assertions))]
            // Definitely not safe, but we'll assume unit tests will catch everything.
            _ => unsafe { std::hint::unreachable_unchecked() },

            #[cfg(debug_assertions)]
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_strategy::proptest;

    #[test]
    #[should_panic]
    fn assuming_none_panics() {
        None.assume()
    }

    #[proptest]
    #[should_panic]
    fn assuming_err_panics(i: i8) {
        Err(i).assume()
    }
}
