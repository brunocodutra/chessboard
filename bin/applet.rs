use anyhow::Error as Anyhow;
use clap::Subcommand;

mod analyze;
mod uci;

#[derive(Subcommand)]
pub enum Applet {
    Analyze(analyze::Analyze),
    Uci(uci::Uci),
}

impl Default for Applet {
    fn default() -> Self {
        Applet::Uci(uci::Uci::default())
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
