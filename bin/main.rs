use futures::executor::{block_on, block_on_stream};
use futures::{channel::mpsc, prelude::*};
use lib::uci::Uci;
use std::io::{prelude::*, stdin, stdout, LineWriter};
use std::thread;

fn main() {
    let (mut tx, input) = mpsc::channel(32);
    let (output, rx) = mpsc::channel(32);

    thread::spawn(move || {
        for item in stdin().lock().lines() {
            match item {
                Err(error) => return eprint!("{error}"),
                Ok(line) => {
                    if let Err(error) = block_on(tx.send(line)) {
                        if error.is_disconnected() {
                            break;
                        }
                    }
                }
            }
        }
    });

    thread::spawn(move || {
        let mut stdout = LineWriter::new(stdout().lock());
        for line in block_on_stream(rx) {
            if let Err(error) = writeln!(stdout, "{line}") {
                return eprint!("{error}");
            }
        }
    });

    block_on(Uci::new(input, output).run()).ok();
}
