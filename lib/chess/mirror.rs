/// Trait for types that can be horizontally mirrored.
pub trait Mirror {
    /// This value's mirror.
    fn mirror(&self) -> Self;
}
