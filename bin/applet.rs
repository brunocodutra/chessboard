use anyhow::Error as Anyhow;
use clap::Subcommand;
use derive_more::From;

mod analyze;
mod uci;

#[derive(From, Subcommand)]
pub enum Applet {
    Analyze(analyze::Analyze),
    Uci(uci::Uci),
}

impl Default for Applet {
    fn default() -> Self {
        uci::Uci::default().into()
    }
}

impl Applet {
    pub async fn execute(self) -> Result<(), Anyhow> {
        match self {
            Applet::Analyze(a) => Ok(a.execute().await?),
            Applet::Uci(a) => Ok(a.execute().await?),
        }
    }
}
