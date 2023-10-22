use clap::Parser;
use lib::uci::Uci;
use std::io;

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Cli {
    #[arg(skip)]
    server: Uci,
}

impl Cli {
    fn run(mut self) -> io::Result<()> {
        self.server.run()
    }
}

fn main() {
    if let Err(e) = Cli::parse().run() {
        panic!("{}", e);
    }
}
