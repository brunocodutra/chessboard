use futures::executor::block_on;
use futures::{channel::mpsc::unbounded, sink::unfold};
use lib::uci::Uci;
use std::io::{prelude::*, stdin, stdout};
use std::{future::ready, thread};

fn main() {
    let (tx, rx) = unbounded();

    thread::spawn(move || {
        for line in stdin().lock().lines() {
            if tx.unbounded_send(line.unwrap()).is_err() {
                break;
            }
        }
    });

    let mut stdout = stdout().lock();
    let output = unfold((), |_, line: String| ready(writeln!(stdout, "{line}")));
    block_on(Uci::new(rx, output).run()).unwrap();
}
