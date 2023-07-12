use anyhow::Error as Anyhow;
use clap::Subcommand;

mod analyze;
mod eval;
mod uci;

#[derive(Subcommand)]
pub enum Applet {
    Analyze(analyze::Analyze),
    Eval(eval::Eval),
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
            Applet::Analyze(a) => a.execute().await,
            Applet::Eval(a) => a.execute().await,
            Applet::Uci(a) => a.execute().await,
        }
    }
}
