use num_traits::{PrimInt, Signed};

/// Trait for integer bounds.
pub trait Bounds {
    /// The equivalent primitive integer
    type Integer: PrimInt + Signed;

    /// The lower bound.
    const LOWER: Self::Integer;

    /// The upper bound.
    const UPPER: Self::Integer;
}
