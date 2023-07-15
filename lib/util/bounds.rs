use num_traits::{AsPrimitive, PrimInt};

/// Trait for integer bounds.
pub trait Bounds {
    /// The equivalent primitive integer
    type Integer: PrimInt + Into<i32> + AsPrimitive<i32>;

    /// The lower bound.
    const LOWER: Self::Integer;

    /// The upper bound.
    const UPPER: Self::Integer;
}
