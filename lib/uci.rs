use crate::chess::{Color, Move, Perspective};
use crate::nnue::Evaluator;
use crate::search::{Engine, HashSize, Limits, Options, Score, ThreadCount};
use crate::util::{Assume, Integer, Trigger};
use arrayvec::ArrayString;
use derive_more::{Deref, Display};
use std::fmt::Debug;
use std::io::{self, stdin, stdout, BufRead, StdinLock, StdoutLock, Write};
use std::time::{Duration, Instant};

#[cfg(test)]
use proptest::prelude::*;

#[derive(Debug, Display, Default, Clone, Eq, PartialEq, Hash, Deref)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
struct UciMove(
    #[deref(forward)]
    #[cfg_attr(test, map(|m: Move| ArrayString::from(&m.to_string()).unwrap()))]
    ArrayString<5>,
);

impl From<Move> for UciMove {
    fn from(m: Move) -> Self {
        Self(ArrayString::from(&m.to_string()).assume())
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
#[cfg_attr(test, arbitrary(args = I,
    bound(I: 'static + Debug + Default + Clone, O: 'static + Debug + Default + Clone)))]
pub struct Uci<I: BufRead, O: Write> {
    #[cfg_attr(test, strategy(Just(args.clone())))]
    input: I,
    #[cfg_attr(test, strategy(Just(O::default())))]
    output: O,
    engine: Engine,
    options: Options,
    position: Evaluator,
    moves: Vec<UciMove>,
}

impl Default for Uci<StdinLock<'_>, StdoutLock<'_>> {
    fn default() -> Self {
        Self::new(stdin().lock(), stdout().lock())
    }
}

impl<I: BufRead, O: Write> Uci<I, O> {
    /// Constructs a new uci server instance.
    pub fn new(input: I, output: O) -> Self {
        Self {
            input,
            output,
            engine: Engine::default(),
            options: Options::default(),
            position: Evaluator::default(),
            moves: Vec::with_capacity(128),
        }
    }

    /// Runs the UCI server.
    pub fn run(&mut self) -> io::Result<()> {
        while let Some(line) = (&mut self.input).lines().next().transpose()? {
            self.output.flush()?;
            if !self.process(&line)? {
                break;
            };
        }

        Ok(())
    }

    fn play(&mut self, uci: &str) {
        if !(0..=5).contains(&uci.len()) || !uci.is_ascii() {
            return eprintln!("invalid move `{uci}`");
        };

        let mut moves = self.position.moves().flatten();
        let Some(m) = moves.find(|&m| UciMove::from(m) == uci) else {
            return eprintln!("illegal move `{uci}` in position `{}`", self.position);
        };

        self.position.play(m);
        self.moves.push(UciMove::from(m));
    }

    fn go(&mut self, limits: Limits) -> io::Result<()> {
        let interrupter = Trigger::armed();
        let pv = self.engine.search(&self.position, limits, &interrupter);
        let best = pv.best().expect("the engine failed to find a move");

        match pv.score().mate() {
            Some(p) if p > 0 => write!(self.output, "info score mate {}", (p + 1).get() / 2),
            Some(p) => write!(self.output, "info score mate {}", (p - 1).get() / 2),
            None => write!(self.output, "info score cp {}", pv.score().get()),
        }?;

        writeln!(self.output, " pv {best}")?;
        writeln!(self.output, "bestmove {best}")?;

        Ok(())
    }

    fn bench(&mut self, limits: Limits) -> io::Result<()> {
        let interrupter = Trigger::armed();
        let timer = Instant::now();
        self.engine.search(&self.position, limits, &interrupter);
        let elapsed = timer.elapsed();
        write!(self.output, "info time {}", elapsed.as_millis())?;

        match limits {
            Limits::Depth(d) => writeln!(self.output, " depth {}", d.get())?,
            Limits::Nodes(nodes) => {
                let nps = nodes as f64 / elapsed.as_secs_f64();
                writeln!(self.output, " nodes {nodes} nps {nps:.0}")?
            }
            _ => writeln!(self.output)?,
        }

        Ok(())
    }

    /// Processes one [`UciMessage`].
    fn process(&mut self, line: &str) -> io::Result<bool> {
        let tokens: Vec<_> = line.split_whitespace().collect();

        match &tokens[..] {
            [] => Ok(true),

            ["position", "startpos", "moves", m, n] => {
                self.position = Evaluator::default();
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
                self.position = Evaluator::default();
                moves.iter().for_each(|&m| self.play(m));
                Ok(true)
            }

            ["position", "startpos"] => {
                self.moves.clear();
                self.position = Evaluator::default();
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
                        self.go(Limits::Clock(t, i))?
                    }
                }

                Ok(true)
            }

            ["go", "depth", depth] => {
                match depth.parse::<u8>() {
                    Ok(d) => self.go(Limits::Depth(d.saturate()))?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go", "nodes", nodes] => {
                match nodes.parse::<u64>() {
                    Ok(n) => self.go(n.into())?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go", "movetime", time] => {
                match time.parse() {
                    Ok(ms) => self.go(Duration::from_millis(ms).into())?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go"] | ["go", "infinite"] => {
                self.go(Limits::None)?;
                Ok(true)
            }

            ["bench", "depth", depth] => {
                match depth.parse::<u8>() {
                    Ok(d) => self.bench(Limits::Depth(d.saturate()))?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["bench", "nodes", nodes] => {
                match nodes.parse::<u64>() {
                    Ok(n) => self.bench(n.into())?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["eval"] => {
                let pos = &self.position;
                let turn = self.position.turn();
                let material: Score = pos.material().evaluate().perspective(turn).saturate();
                let positional: Score = pos.positional().evaluate().perspective(turn).saturate();
                let evaluation: Score = pos.evaluate().perspective(turn).saturate();

                writeln!(
                    self.output,
                    "info material {material} positional {positional} value {evaluation}"
                )?;

                Ok(true)
            }

            ["uci"] => {
                writeln!(self.output, "id name {}", env!("CARGO_PKG_NAME"))?;
                writeln!(self.output, "id author {}", env!("CARGO_PKG_AUTHORS"))?;

                writeln!(
                    self.output,
                    "option name Hash type spin default {} min {} max {}",
                    HashSize::default(),
                    HashSize::lower(),
                    HashSize::upper()
                )?;

                writeln!(
                    self.output,
                    "option name Threads type spin default {} min {} max {}",
                    ThreadCount::default(),
                    ThreadCount::lower(),
                    ThreadCount::upper()
                )?;

                writeln!(self.output, "uciok")?;

                Ok(true)
            }

            ["ucinewgame"] => {
                self.engine = Engine::with_options(self.options);
                self.position = Evaluator::default();
                self.moves.clear();
                Ok(true)
            }

            ["isready"] => {
                writeln!(self.output, "readyok")?;
                Ok(true)
            }

            ["setoption", "name", "hash", "value", hash]
            | ["setoption", "name", "Hash", "value", hash] => {
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

            ["setoption", "name", "threads", "value", threads]
            | ["setoption", "name", "Threads", "value", threads] => {
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

            cmd => {
                eprintln!("ignored unsupported command `{}`", cmd.join(" "));
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::Depth;
    use proptest::sample::Selector;
    use std::{io::Cursor, str};
    use test_strategy::proptest;

    type MockUci = Uci<Cursor<String>, Vec<u8>>;

    impl Default for MockUci {
        fn default() -> Self {
            Self::new(Cursor::new(String::new()), Vec::new())
        }
    }

    #[proptest]
    fn play_updates_position(
        #[filter(#pos.outcome().is_none())] mut pos: Evaluator,
        #[map(|sq: Selector| sq.select(#pos.moves().flatten()))] m: Move,
    ) {
        let mut uci = MockUci {
            position: pos.clone(),
            ..MockUci::default()
        };

        pos.play(m);
        uci.play(&m.to_string());
        assert_eq!(uci.position, pos);
    }

    #[proptest]
    fn play_updates_move_history(
        ms: Vec<UciMove>,
        #[filter(#pos.outcome().is_none())] pos: Evaluator,
        #[map(|sq: Selector| sq.select(#pos.moves().flatten()))] m: Move,
    ) {
        let mut uci = MockUci {
            moves: ms.clone(),
            position: pos.clone(),
            ..MockUci::default()
        };

        uci.play(&m.to_string());
        assert_eq!(uci.moves, [&*ms, &[m.into()]].concat());
    }

    #[proptest]
    fn play_ignores_illegal_move(
        ms: Vec<UciMove>,
        pos: Evaluator,
        #[filter(!#pos.moves().flatten().any(|m| (m.whence(), m.whither()) == (#m.whence(), #m.whither())))]
        m: Move,
    ) {
        let mut uci = MockUci {
            moves: ms.clone(),
            position: pos.clone(),
            ..MockUci::default()
        };

        uci.play(&m.to_string());
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, ms);
    }

    #[proptest]
    fn play_ignores_invalid_move(
        ms: Vec<UciMove>,
        pos: Evaluator,
        #[filter(!(4..5).contains(&#m.len()) || !#m.is_ascii())] m: String,
    ) {
        let mut uci = MockUci {
            position: pos.clone(),
            moves: ms.clone(),
            ..MockUci::default()
        };

        uci.play(&m.to_string());
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, ms);
    }

    #[proptest]
    fn handles_position_with_startpos(
        #[any(Cursor::new("position startpos".to_string()))] mut uci: MockUci,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.position, Evaluator::default());
        assert!(uci.moves.is_empty());
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_position_with_fen(
        #[any(Cursor::new(format!("position fen {}", #pos)))] mut uci: MockUci,
        pos: Evaluator,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.position.to_string(), pos.to_string());
        assert!(uci.moves.is_empty());
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_invalid_fen(
        #[any(Cursor::new(format!("position fen {}", #_s)))] mut uci: MockUci,
        #[filter(#_s.parse::<Evaluator>().is_err())] _s: String,
    ) {
        let pos = uci.position.clone();
        let moves = uci.moves.clone();
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, moves);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_position_with_moves(#[strategy(..=4usize)] n: usize, selector: Selector) {
        let mut uci = MockUci::default();
        let mut input = String::new();
        let mut pos = Evaluator::default();
        let mut moves = vec![];

        input.push_str("position startpos moves");

        for _ in 0..n {
            let m = selector.select(pos.moves().flatten());
            input.push(' ');
            input.push_str(&m.to_string());
            moves.push(UciMove::from(m));
            pos.play(m);
        }

        uci.input = Cursor::new(input);
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, moves);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_go_time_left(
        #[filter(#uci.position.outcome().is_none())]
        #[any(Cursor::new(format!("go wtime {} btime {} winc {} binc {}", #_wt, #_bt, #_wi, #_bi)))]
        mut uci: MockUci,
        #[strategy(..10u8)] _wt: u8,
        #[strategy(..10u8)] _wi: u8,
        #[strategy(..10u8)] _bt: u8,
        #[strategy(..10u8)] _bi: u8,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains("bestmove"));
    }

    #[proptest]
    fn handles_go_depth(
        #[filter(#uci.position.outcome().is_none())]
        #[any(Cursor::new(format!("go depth {}", #_d.get())))]
        mut uci: MockUci,
        _d: Depth,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains("bestmove"));
    }

    #[proptest]
    fn handles_go_nodes(
        #[filter(#uci.position.outcome().is_none())]
        #[any(Cursor::new(format!("go nodes {}", #_n)))]
        mut uci: MockUci,
        #[strategy(..1000u64)] _n: u64,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains("bestmove"));
    }

    #[proptest]
    fn handles_go_time(
        #[filter(#uci.position.outcome().is_none())]
        #[any(Cursor::new(format!("go movetime {}", #_ms)))]
        mut uci: MockUci,
        #[strategy(..10u8)] _ms: u8,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains("bestmove"));
    }

    #[proptest]
    fn handles_eval(#[any(Cursor::new("eval".to_string()))] mut uci: MockUci) {
        let pos = uci.position.clone();
        let value = match pos.turn() {
            Color::White => pos.evaluate(),
            Color::Black => -pos.evaluate(),
        };

        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains(&format!("value {:+}", value.get())));
    }

    #[proptest]
    fn handles_uci(#[any(Cursor::new("uci".to_string()))] mut uci: MockUci) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains("uciok"));
    }

    #[proptest]
    fn handles_new_game(#[any(Cursor::new("ucinewgame".to_string()))] mut uci: MockUci) {
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.position, Evaluator::default());
        assert!(uci.moves.is_empty());
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_option_hash(
        #[any(Cursor::new(format!("setoption name Hash value {}", #h)))] mut uci: MockUci,
        h: HashSize,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.options.hash, h >> 20 << 20);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_invalid_hash_size(
        #[any(Cursor::new(format!("setoption name Hash value {}", #_s)))] mut uci: MockUci,
        #[filter(#_s.trim().parse::<HashSize>().is_err())] _s: String,
    ) {
        let o = uci.options;
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.options, o);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_option_threads(
        #[any(Cursor::new(format!("setoption name Threads value {}", #t)))] mut uci: MockUci,
        t: ThreadCount,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.options.threads, t);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_invalid_thread_count(
        #[any(Cursor::new(format!("setoption name Threads value {}", #_s)))] mut uci: MockUci,
        #[filter(#_s.trim().parse::<ThreadCount>().is_err())] _s: String,
    ) {
        let o = uci.options;
        assert_eq!(uci.run().ok(), Some(()));
        assert_eq!(uci.options, o);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_ready(#[any(Cursor::new("isready".to_string()))] mut uci: MockUci) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(str::from_utf8(&uci.output)?.contains("readyok"));
    }

    #[proptest]
    fn handles_quit(#[any(Cursor::new("quit".to_string()))] mut uci: MockUci) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_unsupported_messages(
        #[any(Cursor::new(#_s))] mut uci: MockUci,
        #[strategy("[^pgbeuisq].*")] _s: String,
    ) {
        assert_eq!(uci.run().ok(), Some(()));
        assert!(uci.output.is_empty());
    }
}
