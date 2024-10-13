use crate::chess::{Color, Move, Perspective};
use crate::nnue::Evaluator;
use crate::search::{Engine, HashSize, Limits, Options, Score, ThreadCount};
use crate::util::{Assume, Integer, Trigger};
use arrayvec::ArrayString;
use derive_more::{Deref, Display};
use futures::{channel::oneshot, future::FusedFuture, prelude::*, select_biased as select};
use std::time::{Duration, Instant};
use std::{fmt::Debug, mem::transmute, thread};

#[cfg(test)]
use proptest::prelude::*;

/// Runs the provided closure on a thread where blocking is acceptable.
///
/// # Safety
///
/// Must be awaited on through completion strictly before any
/// of the variables `f` may capture is dropped.
#[must_use]
unsafe fn unblock<'a, F, R>(f: F) -> impl FusedFuture<Output = R> + 'a
where
    F: FnOnce() -> R + Send + 'a,
    R: Send + 'a,
{
    let (tx, rx) = oneshot::channel();
    thread::spawn(transmute::<
        Box<dyn FnOnce() + Send + 'a>,
        Box<dyn FnOnce() + Send + 'static>,
    >(Box::new(move || tx.send(f()).assume()) as _));
    rx.map(Assume::assume)
}

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

/// A basic UCI server.
#[derive(Debug, Default)]
#[cfg_attr(test, derive(test_strategy::Arbitrary))]
#[cfg_attr(test, arbitrary(args = I,
    bound(I: 'static + Debug + Default + Clone, O: 'static + Debug + Default + Clone)))]
pub struct Uci<I, O> {
    #[cfg_attr(test, strategy(Just(args.clone())))]
    input: I,
    #[cfg_attr(test, strategy(Just(O::default())))]
    output: O,
    engine: Engine,
    options: Options,
    position: Evaluator,
    moves: Vec<UciMove>,
}

impl<I, O> Uci<I, O> {
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
}

impl<I: Stream<Item = String> + Unpin, O: Sink<String> + Unpin> Uci<I, O> {
    /// Runs the UCI server.
    pub async fn run(&mut self) -> Result<(), O::Error> {
        while let Some(line) = self.input.next().await {
            if !self.process(&line).await? {
                break;
            };
        }

        Ok(())
    }

    fn play(&mut self, uci: &str) {
        let mut moves = self.position.moves().flatten();
        let Some(m) = moves.find(|&m| UciMove::from(m) == uci) else {
            return if !(0..=5).contains(&uci.len()) || !uci.is_ascii() {
                eprintln!("invalid move `{uci}`")
            } else {
                eprintln!("illegal move `{uci}` in position `{}`", self.position)
            };
        };

        self.position.play(m);
        self.moves.push(UciMove::from(m));
    }

    async fn go(&mut self, limits: &Limits) -> Result<(), O::Error> {
        let interrupter = Trigger::armed();

        let mut search =
            unsafe { unblock(|| self.engine.search(&self.position, limits, &interrupter)) };

        let stop = async {
            loop {
                match self.input.next().await.as_deref().map(str::trim) {
                    None => break false,
                    Some("stop") => break interrupter.disarm(),
                    Some(cmd) => eprintln!("ignored unsupported command `{cmd}` during search"),
                };
            }
        };

        let pv = select! {
            pv = search => pv,
            _ = stop.fuse() => search.await
        };

        let best = pv.best().expect("the engine failed to find a move");
        let info = match pv.score().mate() {
            Some(p) if p > 0 => format!("info score mate {} pv {best}", (p + 1).get() / 2),
            Some(p) => format!("info score mate {} pv {best}", (p - 1).get() / 2),
            None => format!("info score cp {} pv {best}", pv.score().get()),
        };

        self.output.send(info).await?;
        self.output.send(format!("bestmove {best}")).await?;

        Ok(())
    }

    async fn bench(&mut self, limits: &Limits) -> Result<(), O::Error> {
        let interrupter = Trigger::armed();
        let timer = Instant::now();
        self.engine.search(&self.position, limits, &interrupter);
        let millis = timer.elapsed().as_millis();

        let info = match limits {
            Limits::Depth(d) => format!("info time {millis} depth {}", d.get()),
            Limits::Nodes(nodes) => format!(
                "info time {millis} nodes {nodes} nps {}",
                *nodes as u128 * 1000 / millis
            ),
            _ => return Ok(()),
        };

        self.output.send(info).await
    }

    /// Processes one [`UciMessage`].
    async fn process(&mut self, line: &str) -> Result<bool, O::Error> {
        let tokens: Vec<_> = line.split_whitespace().collect();

        match &tokens[..] {
            [] => Ok(true),
            ["stop"] => Ok(true),
            ["quit"] => Ok(false),

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
                        self.go(&Limits::Clock(t, i)).await?;
                    }
                }

                Ok(true)
            }

            ["go", "depth", depth] => {
                match depth.parse::<u8>() {
                    Ok(d) => self.go(&Limits::Depth(d.saturate())).await?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go", "nodes", nodes] => {
                match nodes.parse::<u64>() {
                    Ok(n) => self.go(&n.into()).await?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go", "movetime", time] => {
                match time.parse() {
                    Ok(ms) => self.go(&Duration::from_millis(ms).into()).await?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["go"] | ["go", "infinite"] => {
                self.go(&Limits::None).await?;
                Ok(true)
            }

            ["bench", "depth", depth] => {
                match depth.parse::<u8>() {
                    Ok(d) => self.bench(&Limits::Depth(d.saturate())).await?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["bench", "nodes", nodes] => {
                match nodes.parse::<u64>() {
                    Ok(n) => self.bench(&n.into()).await?,
                    Err(e) => eprintln!("{e}"),
                }

                Ok(true)
            }

            ["eval"] => {
                let pos = &self.position;
                let turn = self.position.turn();
                let mat: Score = pos.material().evaluate().perspective(turn).saturate();
                let positional: Score = pos.positional().evaluate().perspective(turn).saturate();
                let value: Score = pos.evaluate().perspective(turn).saturate();
                let info = format!("info material {mat} positional {positional} value {value}");
                self.output.send(info).await?;
                Ok(true)
            }

            ["uci"] => {
                let name = format!("id name {}", env!("CARGO_PKG_NAME"));
                let author = format!("id author {}", env!("CARGO_PKG_AUTHORS"));

                let hash = format!(
                    "option name Hash type spin default {} min {} max {}",
                    HashSize::default(),
                    HashSize::lower(),
                    HashSize::upper()
                );

                let threads = format!(
                    "option name Threads type spin default {} min {} max {}",
                    ThreadCount::default(),
                    ThreadCount::lower(),
                    ThreadCount::upper()
                );

                self.output.send(name).await?;
                self.output.send(author).await?;
                self.output.send(hash).await?;
                self.output.send(threads).await?;
                self.output.send("uciok".to_string()).await?;

                Ok(true)
            }

            ["ucinewgame"] => {
                self.engine = Engine::with_options(&self.options);
                self.position = Evaluator::default();
                self.moves.clear();
                Ok(true)
            }

            ["isready"] => {
                self.output.send("readyok".to_string()).await?;
                Ok(true)
            }

            ["setoption", "name", "hash", "value", hash]
            | ["setoption", "name", "Hash", "value", hash] => {
                match hash.parse::<HashSize>() {
                    Err(e) => eprintln!("{e}"),
                    Ok(h) => {
                        if h != self.options.hash {
                            self.options.hash = h;
                            self.engine = Engine::with_options(&self.options);
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
                            self.engine = Engine::with_options(&self.options);
                        }
                    }
                };

                Ok(true)
            }

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
    use futures::executor::block_on;
    use proptest::sample::Selector;
    use std::task::{Context, Poll};
    use std::{collections::VecDeque, pin::Pin};
    use test_strategy::proptest;

    #[derive(Debug, Default, Clone, Eq, PartialEq)]
    struct StaticStream(VecDeque<String>);

    impl StaticStream {
        fn new(items: impl IntoIterator<Item = impl ToString>) -> Self {
            Self(items.into_iter().map(|s| s.to_string()).collect())
        }
    }

    impl Stream for StaticStream {
        type Item = String;

        fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.0.pop_front())
        }
    }

    type MockUci = Uci<StaticStream, Vec<String>>;

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
        #[any(StaticStream::new(["position startpos"]))] mut uci: MockUci,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.position, Evaluator::default());
        assert!(uci.moves.is_empty());
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_position_with_fen(
        #[any(StaticStream::new([format!("position fen {}", #pos)]))] mut uci: MockUci,
        pos: Evaluator,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.position.to_string(), pos.to_string());
        assert!(uci.moves.is_empty());
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_invalid_fen(
        #[any(StaticStream::new([format!("position fen {}", #_s)]))] mut uci: MockUci,
        #[filter(#_s.parse::<Evaluator>().is_err())] _s: String,
    ) {
        let pos = uci.position.clone();
        let moves = uci.moves.clone();
        assert_eq!(block_on(uci.run()), Ok(()));
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

        uci.input = StaticStream::new([input]);
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.position, pos);
        assert_eq!(uci.moves, moves);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_go_time_left(
        #[filter(#uci.position.outcome().is_none())]
        #[any(StaticStream::new([format!("go wtime {} btime {} winc {} binc {}", #_wt, #_bt, #_wi, #_bi)]))]
        mut uci: MockUci,
        #[strategy(..10u8)] _wt: u8,
        #[strategy(..10u8)] _wi: u8,
        #[strategy(..10u8)] _bt: u8,
        #[strategy(..10u8)] _bi: u8,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().contains("bestmove"));
    }

    #[proptest]
    fn handles_go_depth(
        #[filter(#uci.position.outcome().is_none())]
        #[any(StaticStream::new([format!("go depth {}", #_d.get())]))]
        mut uci: MockUci,
        _d: Depth,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().contains("bestmove"));
    }

    #[proptest]
    fn handles_go_nodes(
        #[filter(#uci.position.outcome().is_none())]
        #[any(StaticStream::new([format!("go nodes {}", #_n)]))]
        mut uci: MockUci,
        #[strategy(..1000u64)] _n: u64,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().contains("bestmove"));
    }

    #[proptest]
    fn handles_go_time(
        #[filter(#uci.position.outcome().is_none())]
        #[any(StaticStream::new([format!("go movetime {}", #_ms)]))]
        mut uci: MockUci,
        #[strategy(..10u8)] _ms: u8,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().contains("bestmove"));
    }

    #[proptest]
    fn handles_go(
        #[by_ref]
        #[filter(#uci.position.outcome().is_none())]
        #[any(StaticStream::new(["go"]))]
        mut uci: MockUci,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().contains("bestmove"));
    }

    #[proptest]
    fn handles_stop_during_search(
        #[by_ref]
        #[filter(#uci.position.outcome().is_none())]
        #[any(StaticStream::new(["go", "stop"]))]
        mut uci: MockUci,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().contains("bestmove"));
    }

    #[proptest]
    fn handles_stop(#[any(StaticStream::new(["stop"]))] mut uci: MockUci) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_quit(#[any(StaticStream::new(["quit"]))] mut uci: MockUci) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_eval(#[any(StaticStream::new(["eval"]))] mut uci: MockUci) {
        let pos = uci.position.clone();
        let value = match pos.turn() {
            Color::White => pos.evaluate(),
            Color::Black => -pos.evaluate(),
        };

        let value = format!("value {:+}", value.get());
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().ends_with(&value));
    }

    #[proptest]
    fn handles_uci(#[any(StaticStream::new(["uci"]))] mut uci: MockUci) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.concat().ends_with("uciok"));
    }

    #[proptest]
    fn handles_new_game(#[any(StaticStream::new(["ucinewgame"]))] mut uci: MockUci) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.position, Evaluator::default());
        assert!(uci.moves.is_empty());
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_option_hash(
        #[any(StaticStream::new([format!("setoption name Hash value {}", #h)]))] mut uci: MockUci,
        h: HashSize,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.options.hash, h >> 20 << 20);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_invalid_hash_size(
        #[any(StaticStream::new([format!("setoption name Hash value {}", #_s)]))] mut uci: MockUci,
        #[filter(#_s.trim().parse::<HashSize>().is_err())] _s: String,
    ) {
        let o = uci.options.clone();
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.options, o);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_option_threads(
        #[any(StaticStream::new([format!("setoption name Threads value {}", #t)]))]
        mut uci: MockUci,
        t: ThreadCount,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.options.threads, t);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn ignores_invalid_thread_count(
        #[any(StaticStream::new([format!("setoption name Threads value {}", #_s)]))]
        mut uci: MockUci,
        #[filter(#_s.trim().parse::<ThreadCount>().is_err())] _s: String,
    ) {
        let o = uci.options.clone();
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.options, o);
        assert!(uci.output.is_empty());
    }

    #[proptest]
    fn handles_ready(#[any(StaticStream::new(["isready"]))] mut uci: MockUci) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert_eq!(uci.output.concat(), "readyok");
    }

    #[proptest]
    fn ignores_unsupported_messages(
        #[any(StaticStream::new([#_s]))] mut uci: MockUci,
        #[strategy("[^pgbeuisq].*")] _s: String,
    ) {
        assert_eq!(block_on(uci.run()), Ok(()));
        assert!(uci.output.is_empty());
    }
}
