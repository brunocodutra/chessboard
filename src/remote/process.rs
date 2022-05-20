use crate::Remote;
use anyhow::{Context, Error as Anyhow};
use async_trait::async_trait;
use derive_more::DebugCustom;
use std::{fmt::Display, io};
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter, Lines,
};
use tokio::{runtime, task::block_in_place};
use tracing::{error, info, instrument, warn};

#[cfg(test)]
use tokio::io::DuplexStream;

#[cfg_attr(test, mockall::automock(type Stdin = DuplexStream; type Stdout = DuplexStream; type Status = String;))]
#[async_trait]
trait Program {
    fn id(&self) -> Option<u32>;

    type Stdin: AsyncWrite;
    fn stdin(&mut self) -> io::Result<Self::Stdin>;

    type Stdout: AsyncRead;
    fn stdout(&mut self) -> io::Result<Self::Stdout>;

    type Status: Display;
    async fn wait(&mut self) -> io::Result<Self::Status>;
}

#[cfg(not(test))]
use std::process::ExitStatus;

#[cfg(not(test))]
use tokio::process::{Child, ChildStdin, ChildStdout};

#[cfg(not(test))]
#[async_trait]
impl Program for Child {
    fn id(&self) -> Option<u32> {
        self.id()
    }

    type Stdin = ChildStdin;
    fn stdin(&mut self) -> io::Result<Self::Stdin> {
        self.stdin.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                Anyhow::msg("failed to open the child process' stdin"),
            )
        })
    }

    type Stdout = ChildStdout;
    fn stdout(&mut self) -> io::Result<Self::Stdout> {
        self.stdout.take().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                Anyhow::msg("failed to open the child process' stdout"),
            )
        })
    }

    type Status = ExitStatus;
    async fn wait(&mut self) -> io::Result<ExitStatus> {
        Self::wait(self).await
    }
}

#[cfg(test)]
type Child = MockProgram;

/// An implementation of trait [`Remote`] as a child process.
///
/// # Warning
/// Dropping this type blocks until the child process exits.
#[derive(DebugCustom)]
#[debug(fmt = "Process({})", "child.id().map(i64::from).unwrap_or(-1)")]
pub struct Process {
    child: Child,
    writer: BufWriter<<Child as Program>::Stdin>,
    reader: Lines<BufReader<<Child as Program>::Stdout>>,
}

impl Process {
    fn new(mut child: Child) -> io::Result<Self> {
        let stdin = child.stdin()?;
        let stdout = child.stdout()?;

        info!(pid = child.id());

        Ok(Process {
            child,
            writer: BufWriter::new(stdin),
            reader: BufReader::new(stdout).lines(),
        })
    }

    /// Spawns a child process.
    #[cfg(not(test))]
    #[instrument(level = "trace", err)]
    pub async fn spawn(program: &str) -> io::Result<Self> {
        Process::new(
            tokio::process::Command::new(program)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()?,
        )
    }
}

/// Flushes the outbound buffer and waits for the child process to exit.
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

        match result.context("failed to gracefully terminate the child process") {
            Ok(s) => info!(pid, "{}", s),
            Err(e) => error!(pid, "{:?}", e),
        }
    }
}

#[async_trait]
impl Remote for Process {
    #[instrument(level = "trace", err)]
    async fn recv(&mut self) -> io::Result<String> {
        use io::ErrorKind::UnexpectedEof;
        Ok(self.reader.next_line().await?.ok_or(UnexpectedEof)?)
    }

    #[instrument(level = "trace", skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> io::Result<()> {
        let msg = item.to_string();
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
        let mut child = MockProgram::new();

        child.expect_id().once().returning(move || id);

        let (stdin, stdout) = duplex(1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        child.expect_stdout().once().return_once(move || Ok(stdout));

        assert!(Process::new(child).is_ok());
    }

    #[proptest]
    fn new_fails_if_stdin_is_not_available(e: io::Error) {
        let mut child = MockProgram::new();

        let kind = e.kind();
        child.expect_stdin().once().return_once(move || Err(e));

        assert_eq!(Process::new(child).unwrap_err().kind(), kind);
    }

    #[proptest]
    fn new_fails_if_stdout_is_not_available(e: io::Error) {
        let mut child = MockProgram::new();

        let kind = e.kind();
        let (stdin, _) = duplex(1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        child.expect_stdout().once().return_once(move || Err(e));

        assert_eq!(Process::new(child).unwrap_err().kind(), kind);
    }

    #[proptest]
    fn drop_gracefully_terminates_child_process(id: Option<u32>, status: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockProgram::new();

        child.expect_id().once().returning(move || id);
        child.expect_wait().once().return_once(move || Ok(status));

        let (stdin, stdout) = duplex(1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        child.expect_stdout().once().return_once(move || Ok(stdout));

        rt.block_on(async move {
            drop(Process::new(child));
        })
    }

    #[proptest]
    fn drop_recovers_from_errors(id: Option<u32>, e: io::Error) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockProgram::new();

        child.expect_id().once().returning(move || id);
        child.expect_wait().once().return_once(move || Err(e));

        let (stdin, stdout) = duplex(1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        child.expect_stdout().once().return_once(move || Ok(stdout));

        rt.block_on(async move {
            drop(Process::new(child));
        })
    }

    #[proptest]
    fn drop_recovers_from_missing_runtime(id: Option<u32>) {
        let mut child = MockProgram::new();

        child.expect_id().once().returning(move || id);

        let (stdin, stdout) = duplex(1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        child.expect_stdout().once().return_once(move || Ok(stdout));

        drop(Process::new(child));
    }

    #[proptest]
    fn recv_waits_for_line_break(id: Option<u32>, #[strategy("[^\r\n]")] s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockProgram::new();

        child.expect_id().once().returning(move || id);

        let (stdin, _) = duplex(1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        let (mut tx, stdout) = duplex(s.len() + 1);
        child.expect_stdout().once().return_once(move || Ok(stdout));

        rt.block_on(tx.write_all(s.as_bytes()))?;
        rt.block_on(tx.write_u8(b'\n'))?;

        let mut process = Process::new(child)?;
        assert_eq!(rt.block_on(process.recv())?, s);
    }

    #[proptest]
    fn send_appends_line_break(id: Option<u32>, s: String) {
        let rt = runtime::Builder::new_multi_thread().build()?;
        let mut child = MockProgram::new();

        child.expect_id().once().returning(move || id);

        let (stdin, mut rx) = duplex(s.len() + 1);
        child.expect_stdin().once().return_once(move || Ok(stdin));
        let (_, stdout) = duplex(1);
        child.expect_stdout().once().return_once(move || Ok(stdout));

        let expected = format!("{}\n", s);

        let mut process = Process::new(child)?;
        rt.block_on(process.send(s))?;
        rt.block_on(process.flush())?;

        let mut buf = vec![0u8; expected.len()];
        rt.block_on(rx.read_exact(&mut buf))?;

        assert_eq!(str::from_utf8(&buf)?, expected);
    }
}
