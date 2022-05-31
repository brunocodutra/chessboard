use crate::Io;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use derive_more::DebugCustom;
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter, Lines};
use tokio::{runtime, task::block_in_place};
use tracing::{error, info, instrument, warn};

#[cfg_attr(test, mockall::automock(
    type Stdin = tokio::io::DuplexStream;
    type Stdout = tokio::io::DuplexStream;
    type Status = String;
))]
#[async_trait]
trait ChildProcess {
    fn id(&self) -> Option<u32>;

    type Stdin;
    type Stdout;
    fn pipe(&mut self) -> io::Result<(Self::Stdin, Self::Stdout)>;

    type Status;
    async fn wait(&mut self) -> io::Result<Self::Status>;
}

#[async_trait]
impl ChildProcess for tokio::process::Child {
    fn id(&self) -> Option<u32> {
        self.id()
    }

    type Stdin = tokio::process::ChildStdin;
    type Stdout = tokio::process::ChildStdout;
    fn pipe(&mut self) -> io::Result<(Self::Stdin, Self::Stdout)> {
        Option::zip(self.stdin.take(), self.stdout.take()).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                Anyhow::msg("failed to open the remote process' stdio"),
            )
        })
    }

    type Status = std::process::ExitStatus;
    async fn wait(&mut self) -> io::Result<Self::Status> {
        self.wait().await
    }
}

#[cfg(test)]
type Child = MockChildProcess;

#[cfg(not(test))]
type Child = tokio::process::Child;

/// An [`Io`] interface for a remote process.
#[derive(DebugCustom)]
#[debug(fmt = "Process({})", "child.id().map(i64::from).unwrap_or(-1)")]
pub struct Process {
    child: Child,
    writer: BufWriter<<Child as ChildProcess>::Stdin>,
    reader: Lines<BufReader<<Child as ChildProcess>::Stdout>>,
}

impl Process {
    fn new(mut child: Child) -> io::Result<Self> {
        let (stdin, stdout) = child.pipe()?;

        info!(pid = child.id());

        Ok(Process {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout).lines(),
        })
    }

    /// Spawns a remote process.
    #[instrument(level = "trace", err, ret)]
    pub async fn spawn(path: &str) -> io::Result<Self> {
        #[cfg(test)]
        {
            let mut child = MockChildProcess::new();
            child.expect_id().returning(|| None);
            child.expect_pipe().returning(|| Ok(tokio::io::duplex(1)));
            child.expect_wait().returning(|| Ok(String::new()));
            Process::new(child)
        }

        #[cfg(not(test))]
        {
            Process::new(
                tokio::process::Command::new(path)
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn()?,
            )
        }
    }
}

/// Flushes the outbound buffer and waits for the remote process to exit.
impl Drop for Process {
    #[instrument(level = "trace")]
    fn drop(&mut self) {
        let pid = self.child.id();

        let result: Result<_, Anyhow> = block_in_place(|| {
            runtime::Handle::try_current()?.block_on(async {
                self.writer.flush().await?;
                Ok(self.child.wait().await?)
            })
        });

        match result.context("failed to gracefully terminate the remote process") {
            Ok(s) => info!(pid, "{}", s),
            Err(e) => error!(pid, "{:?}", e),
        }
    }
}

#[async_trait]
impl Io for Process {
    #[instrument(level = "trace", err, ret)]
    async fn recv(&mut self) -> io::Result<String> {
        use io::ErrorKind::UnexpectedEof;
        Ok(self.reader.next_line().await?.ok_or(UnexpectedEof)?)
    }

    #[instrument(level = "trace", err)]
    async fn send(&mut self, msg: &str) -> io::Result<()> {
        self.writer.write_all(msg.as_bytes()).await?;
        self.writer.write_u8(b'\n').await?;
        Ok(())
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> io::Result<()> {
        self.writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;
    use test_strategy::proptest;
    use tokio::io::{duplex, AsyncReadExt};

    #[proptest]
    fn new_expects_stdin_and_stdout(id: Option<u32>) {
        let mut child = MockChildProcess::new();

        child.expect_id().once().return_once(move || id);

        let pipe = duplex(1);
        child.expect_pipe().once().return_once(move || Ok(pipe));

        assert!(Process::new(child).is_ok());
    }

    #[proptest]
    fn new_fails_if_stdio_not_available(e: io::Error) {
        let mut child = MockChildProcess::new();

        let kind = e.kind();
        child.expect_pipe().once().return_once(move || Err(e));

        assert_eq!(Process::new(child).err().map(|e| e.kind()), Some(kind));
    }

    #[proptest]
    fn drop_gracefully_terminates_child_process(id: Option<u32>, status: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockChildProcess::new();

        child.expect_id().once().return_once(move || id);
        child.expect_wait().once().return_once(move || Ok(status));

        let pipe = duplex(1);
        child.expect_pipe().once().return_once(move || Ok(pipe));

        rt.block_on(async move {
            drop(Process::new(child));
        })
    }

    #[proptest]
    fn drop_recovers_from_errors(id: Option<u32>, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockChildProcess::new();

        child.expect_id().once().return_once(move || id);
        child.expect_wait().once().return_once(move || Err(e));

        let pipe = duplex(1);
        child.expect_pipe().once().return_once(move || Ok(pipe));

        rt.block_on(async move {
            drop(Process::new(child));
        })
    }

    #[proptest]
    fn drop_recovers_from_missing_runtime(id: Option<u32>) {
        let mut child = MockChildProcess::new();

        child.expect_id().once().return_once(move || id);

        let pipe = duplex(1);
        child.expect_pipe().once().return_once(move || Ok(pipe));

        drop(Process::new(child));
    }

    #[proptest]
    fn recv_waits_for_line_break(id: Option<u32>, #[strategy("[^\r\n]")] s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockChildProcess::new();

        child.expect_id().once().return_once(move || id);

        let (stdin, _) = duplex(1);
        let (mut tx, stdout) = duplex(s.len() + 1);
        let pipe = (stdin, stdout);
        child.expect_pipe().once().return_once(move || Ok(pipe));

        rt.block_on(tx.write_all(s.as_bytes()))?;
        rt.block_on(tx.write_u8(b'\n'))?;

        let mut process = Process::new(child)?;
        assert_eq!(rt.block_on(process.recv())?, s);
    }

    #[proptest]
    fn send_appends_line_break(id: Option<u32>, s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockChildProcess::new();

        child.expect_id().once().return_once(move || id);

        let (stdin, mut rx) = duplex(s.len() + 1);
        let (_, stdout) = duplex(1);
        let pipe = (stdin, stdout);
        child.expect_pipe().once().return_once(move || Ok(pipe));

        let expected = format!("{}\n", s);

        let mut process = Process::new(child)?;
        rt.block_on(process.send(&s))?;
        rt.block_on(process.flush())?;

        let mut buf = vec![0u8; expected.len()];
        rt.block_on(rx.read_exact(&mut buf))?;

        assert_eq!(str::from_utf8(&buf)?, expected);
    }
}
