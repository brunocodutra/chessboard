/// Trait for types that can be mirrored.
#[const_trait]
pub trait Mirror {
    /// This value's mirror.
    fn mirror(&self) -> Self;
}
