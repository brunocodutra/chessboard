use anyhow::Error as Anyhow;

/// Trait for types that encode runtime configuration.
pub trait Build {
    /// The type setup from this configuration.
    type Output;

    /// Consume this configuration to setup [`Build::Output`].
    fn build(self) -> Result<Self::Output, Anyhow>;
}
