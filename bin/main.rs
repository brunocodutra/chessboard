use lib::uci::Uci;

fn main() {
    let mut server = Uci::default();
    if let Err(e) = server.run() {
        panic!("{}", e);
    }
}
