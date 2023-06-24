use num_traits::PrimInt;
use std::fmt::{Debug, Display};

/// Trait for integer bounds.
pub trait Bounds {
    /// The equivalent primitive integer
    type Integer: 'static + PrimInt + Debug + Display + Into<i64>;

    /// The lower bound.
    const LOWER: Self::Integer;

    /// The upper bound.
    const UPPER: Self::Integer;
}
