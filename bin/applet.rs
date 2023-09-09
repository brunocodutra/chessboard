use anyhow::Error as Anyhow;
use clap::Subcommand;

mod eval;
mod uci;

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum Applet {
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
            Applet::Eval(a) => a.execute().await,
            Applet::Uci(a) => a.execute().await,
        }
    }
}
