/// Trait for integer bounds.
pub trait Bounds<T> {
    /// The lower bound.
    const LOWER: T;

    /// The upper bound.
    const UPPER: T;
}
