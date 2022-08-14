use anyhow::Error as Anyhow;
use async_trait::async_trait;
use clap::Subcommand;

mod play;
mod search;

/// Trait for types that behave like subcommands.
#[async_trait]
pub trait Execute {
    /// Execute the subcommand.
    async fn execute(self) -> Result<(), Anyhow>;
}

#[derive(Subcommand)]
pub enum Applet {
    Search(search::Search),
    Play(play::Play),
}

#[async_trait]
impl Execute for Applet {
    async fn execute(self) -> Result<(), Anyhow> {
        match self {
            Applet::Search(a) => Ok(a.execute().await?),
            Applet::Play(a) => Ok(a.execute().await?),
        }
    }
}
