use crate::chess::{Color, Move, Position};
use crate::search::{Depth, Engine, HashSize, Limits, Options, Score, ThreadCount};
use crate::{nnue::Evaluator, util::Assume};
use arrayvec::ArrayString;
use derive_more::{Deref, Display};
use std::io::{self, stdin, stdout, Write};
use std::{fmt::Write as _, time::Duration};

#[derive(Debug, Display, Default, Clone, Eq, PartialEq, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
struct UciMove(
    #[deref(forward)]
    #[cfg_attr(test, map(|m: Move| ArrayString::from(&m.to_string()).unwrap()))]
    ArrayString<5>,
);

impl From<Move> for UciMove {
    fn from(m: Move) -> Self {
        let mut uci = ArrayString::new();
        write!(uci, "{m}").assume();
        Self(uci)
    }
}

impl PartialEq<&str> for UciMove {
    fn eq(&self, other: &&str) -> bool {
        self.0.eq(*other)
    }
}

/// A basic *not fully compliant* UCI server.
#[derive(Debug)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
pub struct Uci {
    engine: Engine,
    options: Options,
    position: Position,
    moves: Vec<UciMove>,
}

impl Default for Uci {
    fn default() -> Self {
        Self {
            engine: Engine::default(),
            options: Options::default(),
            position: Position::default(),
            moves: Vec::with_capacity(128),
        }
    }
}

impl Uci {
    fn play(&mut self, uci: &str) {
        if !(0..=5).contains(&uci.len()) || !uci.is_ascii() {
            return eprintln!("invalid move `{uci}`");
        };

        let Some(m) = self.position.moves().find(|&m| UciMove::from(m) == uci) else {
            return eprintln!("illegal move `{uci}` in position `{}`", self.position);
        };

        self.position.play(m);
        self.moves.push(UciMove::from(m));
    }

    fn go<W: Write>(&mut self, limits: Limits, out: &mut W) -> io::Result<()> {
        let pv = self.engine.search(&self.position, limits);
        let best = *pv.first().expect("the engine failed to find a move");

        match pv.score().mate() {
            Some(p) if p > 0 => write!(out, "info score mate {}", (p + 1).get() / 2),
            Some(p) => write!(out, "info score mate {}", (p - 1).get() / 2),
            None => write!(out, "info score cp {}", pv.score().get()),
        }?;

        write!(out, " pv")?;

        for m in pv {
            write!(out, " {m}")?;
        }

        writeln!(out)?;
        writeln!(out, "bestmove {best}")?;

        Ok(())
    }

    /// Processes one [`UciMessage`].
    pub fn process<W: Write>(&mut self, msg: &str, out: &mut W) -> io::Result<bool> {
        let tokens: Vec<_> = msg.split_whitespace().collect();

        match &tokens[..] {
            [] => Ok(true),

            ["position", "startpos", "moves", m, n] => {
                self.position = Position::default();
                self.play(m);
                self.play(n);
                Ok(true)
            }

            ["position", "startpos", "moves", moves @ .., m, n] if self.moves == moves => {
                self.play(m);
                self.play(n);
                Ok(true)
            }

            ["position", "startpos", "moves", moves @ ..] => {
                self.moves.clear();
                self.position = Position::default();
                moves.iter().for_each(|&m| self.play(m));
                Ok(true)
            }

            ["position", "startpos"] => {
                self.moves.clear();
                self.position = Position::default();
                Ok(true)
            }

            ["position", "fen", fen @ ..] => {
                match fen.join(" ").parse() {
                    Err(e) => eprintln!("{e}"),
                    Ok(pos) => {
                        self.moves.clear();
                        self.position = pos;
                    }
                }

                Ok(true)
            }

            ["go", "wtime", wtime, "btime", btime, "winc", winc, "binc", binc]
            | ["go", "wtime", wtime, "winc", winc, "btime", btime, "binc", binc] => {
                let (t, i) = match self.position.turn() {
                    Color::White => (wtime, winc),
                    Color::Black => (btime, binc),
                };

                match (t.parse(), i.parse()) {
                    (Err(e), _) | (_, Err(e)) => eprintln!("{e}"),
                    (Ok(t), Ok(i)) => {
                        let t = Duration::from_millis(t);
                        let i = Duration::from_millis(i);
                        self.go(Limits::Clock(t, i), out)?
                    }
                }

                Ok(true)
            }

            ["go", "depth", depth] => {
                match depth.parse::<u8>() {
                    Ok(d) => self.go(Depth::saturate(d).into(), out)?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go", "nodes", depth] => {
                match depth.parse::<u64>() {
                    Ok(n) => self.go(n.into(), out)?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go", "movetime", time] => {
                match time.parse() {
                    Ok(ms) => self.go(Duration::from_millis(ms).into(), out)?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go"] | ["go", "infinite"] => {
                self.go(Limits::None, out)?;
                Ok(true)
            }

            ["eval"] => {
                let pos = Evaluator::new(self.position.clone());
                let [material, positional, value]: [Score; 3] = match pos.turn() {
                    Color::White => [
                        pos.material().evaluate().cast(),
                        pos.positional().evaluate().cast(),
                        pos.evaluate().cast(),
                    ],

                    Color::Black => [
                        -pos.material().evaluate().cast(),
                        -pos.positional().evaluate().cast(),
                        -pos.evaluate().cast(),
                    ],
                };

                writeln!(
                    out,
                    "info material {material} positional {positional} value {value}"
                )?;

                Ok(true)
            }

            ["uci"] => {
                writeln!(out, "id name {}", env!("CARGO_PKG_NAME"))?;
                writeln!(out, "id author {}", env!("CARGO_PKG_AUTHORS"))?;

                writeln!(
                    out,
                    "option name Hash type spin default {} min {} max {}",
                    HashSize::default(),
                    HashSize::min(),
                    HashSize::max()
                )?;

                writeln!(
                    out,
                    "option name Threads type spin default {} min {} max {}",
                    ThreadCount::default(),
                    ThreadCount::min(),
                    ThreadCount::max()
                )?;

                writeln!(out, "uciok")?;

                Ok(true)
            }

            ["ucinewgame"] => {
                self.engine = Engine::with_options(self.options);
                self.position = Position::default();
                self.moves.clear();
                Ok(true)
            }

            ["isready"] => {
                writeln!(out, "readyok")?;
                Ok(true)
            }

            ["setoption", "name", "Hash", "value", hash] => {
                match hash.parse::<HashSize>() {
                    Err(e) => eprintln!("{e}"),
                    Ok(h) => {
                        if h != self.options.hash {
                            self.options.hash = h;
                            self.engine = Engine::with_options(self.options);
                        }
                    }
                }

                Ok(true)
            }

            ["setoption", "name", "Threads", "value", threads] => {
                match threads.parse::<ThreadCount>() {
                    Err(e) => eprintln!("{e}"),
                    Ok(t) => {
                        if t != self.options.threads {
                            self.options.threads = t;
                            self.engine = Engine::with_options(self.options);
                        }
                    }
                };

                Ok(true)
            }

            ["quit"] => Ok(false),

            msg => {
                eprintln!("ignored unsupported command `{}`", msg.join(" "));
                Ok(true)
            }
        }
    }

    /// Runs the UCI server.
    pub fn run(&mut self) -> io::Result<()> {
        for request in stdin().lines() {
            let mut stdout = stdout().lock();
            if !self.process(&request?, &mut stdout)? {
                break;
            };

            stdout.flush()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::Move;
    use proptest::sample::Selector;
    use std::str;
    use test_strategy::proptest;

    #[proptest]
    fn play_updates_position(
        #[filter(#pos.outcome().is_none())] mut pos: Position,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        let mut uci = Uci {
            position: pos.clone(),
            ..Uci::default()
        };

        pos.play(m);
        uci.play(&m.to_string());
        assert_eq!(uci.position, pos);
    }

    #[proptest]
    fn play_updates_move_history(
        ms: Vec<UciMove>,
        #[filter(#pos.outcome().is_none())] pos: Position,
        #[map(|s: Selector| s.select(#pos.moves()))] m: Move,
    ) {
        let mut uci = Uci {
            moves: ms.clone(),
            position: pos.clone(),
            ..Uci::default()
        };

        uci.play(&m.to_string());
        assert_eq!(uci.moves, [&*ms, &[m.into()]].concat());
    }

    #[proptest]
    fn play_ignores_illegal_move(
        ms: Vec<UciMove>,
        #[by_ref] pos: Position,
        #[filter(!#pos.moves().any(|m| (m.whence(), m.whither()) == (#m.whence(), #m.whither())))]
        m: Move,
    ) {
        let mut uci = Uci {
            moves: ms.clone(),
            position: pos.clone(),
            ..Uci::default()
        };

        uci.play(&m.to_string());
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, ms);
    }

    #[proptest]
    fn play_ignores_invalid_move(
        ms: Vec<UciMove>,
        pos: Position,
        #[filter(!(4..5).contains(&#m.len()) || !#m.is_ascii())] m: String,
    ) {
        let mut uci = Uci {
            position: pos.clone(),
            moves: ms.clone(),
            ..Uci::default()
        };

        uci.play(&m.to_string());
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, ms);
    }

    #[proptest]
    fn process_handles_position_with_startpos(mut uci: Uci) {
        let mut out = vec![];
        assert!(uci.process("position startpos", &mut out)?);
        assert_eq!(uci.position, Position::default());
        assert!(uci.moves.is_empty());
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_handles_position_with_fen(mut uci: Uci, pos: Position) {
        let mut out = vec![];
        assert!(uci.process(&format!("position fen {pos}"), &mut out)?);
        assert_eq!(uci.position, pos);
        assert!(uci.moves.is_empty());
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_ignores_invalid_fen(
        mut uci: Uci,
        #[filter(#s.parse::<Position>().is_err())] s: String,
    ) {
        let mut out = vec![];
        let pos = uci.position.clone();
        let moves = uci.moves.clone();
        assert!(uci.process(&format!("position fen {s}"), &mut out)?);
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, moves);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_handles_position_with_moves(#[strategy(..=4usize)] n: usize, selector: Selector) {
        let mut out = vec![];
        let mut uci = Uci::default();
        let mut pos = Position::default();
        let mut moves = vec![];
        let mut msg = "position startpos moves".to_string();

        for _ in 0..n {
            let m = selector.select(pos.moves());
            write!(msg, " {m}")?;
            moves.push(UciMove::from(m));
            pos.play(m);
        }

        assert!(uci.process(&msg, &mut out)?);
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, moves);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_handles_go_time_left(
        #[strategy(..10u8)] wt: u8,
        #[strategy(..10u8)] wi: u8,
        #[strategy(..10u8)] bt: u8,
        #[strategy(..10u8)] bi: u8,
    ) {
        let mut out = vec![];
        let mut uci = Uci::default();
        let msg = format!("go wtime {wt} btime {bt} winc {wi} binc {bi}");
        assert!(uci.process(&msg, &mut out)?);
        assert!(str::from_utf8(&out)?.contains("bestmove"));
    }

    #[proptest]
    fn process_handles_go_depth(d: Depth) {
        let mut out = vec![];
        let mut uci = Uci::default();
        assert!(uci.process(&format!("go depth {}", d.get()), &mut out)?);
        assert!(str::from_utf8(&out)?.contains("bestmove"));
    }

    #[proptest]
    fn process_handles_go_nodes(#[strategy(..1000u64)] n: u64) {
        let mut out = vec![];
        let mut uci = Uci::default();
        assert!(uci.process(&format!("go nodes {n}"), &mut out)?);
        assert!(str::from_utf8(&out)?.contains("bestmove"));
    }

    #[proptest]
    fn process_handles_go_time(#[strategy(..10u8)] ms: u8) {
        let mut out = vec![];
        let mut uci = Uci::default();
        assert!(uci.process(&format!("go movetime {ms}"), &mut out)?);
        assert!(str::from_utf8(&out)?.contains("bestmove"));
    }

    #[proptest]
    fn process_handles_eval(mut uci: Uci) {
        let mut out = vec![];
        let pos = Evaluator::new(uci.position.clone());
        let value = match pos.turn() {
            Color::White => pos.evaluate(),
            Color::Black => -pos.evaluate(),
        };

        assert!(uci.process("eval", &mut out)?);
        assert!(str::from_utf8(&out)?.contains(&format!("value {:+}", value.get())));
    }

    #[proptest]
    fn process_handles_uci(mut uci: Uci) {
        let mut out = vec![];
        assert!(uci.process("uci", &mut out)?);
        assert!(str::from_utf8(&out)?.contains("uciok"));
    }

    #[proptest]
    fn process_handles_new_game(mut uci: Uci) {
        let mut out = vec![];
        assert!(uci.process("ucinewgame", &mut out)?);
        assert_eq!(uci.position, Position::default());
        assert!(uci.moves.is_empty());
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_handles_option_hash(mut uci: Uci, h: HashSize) {
        let mut out = vec![];
        assert!(uci.process(&format!("setoption name Hash value {h}"), &mut out)?);
        assert_eq!(uci.options.hash, h >> 20 << 20);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_ignores_invalid_hash_size(
        mut uci: Uci,
        #[filter(#s.trim().parse::<HashSize>().is_err())] s: String,
    ) {
        let mut out = vec![];
        let o = uci.options;
        assert!(uci.process(&format!("setoption name Hash value {s}"), &mut out)?);
        assert_eq!(uci.options, o);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_handles_option_threads(mut uci: Uci, t: ThreadCount) {
        let mut out = vec![];
        assert!(uci.process(&format!("setoption name Threads value {t}"), &mut out)?);
        assert_eq!(uci.options.threads, t);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_ignores_invalid_thread_count(
        mut uci: Uci,
        #[filter(#s.trim().parse::<ThreadCount>().is_err())] s: String,
    ) {
        let mut out = vec![];
        let o = uci.options;
        assert!(uci.process(&format!("setoption name Threads value {s}"), &mut out)?);
        assert_eq!(uci.options, o);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_handles_ready(mut uci: Uci) {
        let mut out = vec![];
        assert!(uci.process("isready", &mut out)?);
        assert!(str::from_utf8(&out)?.contains("readyok"));
    }

    #[proptest]
    fn process_handles_quit(mut uci: Uci) {
        let mut out = vec![];
        assert!(!uci.process("quit", &mut out)?);
        assert!(out.is_empty());
    }

    #[proptest]
    fn process_ignores_unsupported_messages(mut uci: Uci, #[strategy(".*[^pgeuisq].*")] s: String) {
        let mut out = vec![];
        assert!(uci.process(&s, &mut out)?);
        assert!(out.is_empty());
    }
}
