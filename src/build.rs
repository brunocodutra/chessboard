use anyhow::Error as Anyhow;
use async_trait::async_trait;

/// Trait for types that encode runtime configuration.
#[async_trait]
pub trait Build {
    /// The type setup from this configuration.
    type Output;

    /// Consume this configuration to setup [`Build::Output`].
    async fn build(self) -> Result<Self::Output, Anyhow>;
}
