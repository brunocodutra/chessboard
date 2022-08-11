/// Trait for types that build other types.
pub trait Build {
    /// The type to be built.
    type Output;

    /// The reason why [`Build::Output`] could not be built.
    type Error;

    /// Build an instance of [`Build::Output`].
    fn build(self) -> Result<Self::Output, Self::Error>;
}
