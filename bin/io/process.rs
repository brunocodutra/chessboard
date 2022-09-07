use super::{Io, Pipe};
use anyhow::{bail, Context, Error as Anyhow};
use async_trait::async_trait;
use std::{io, time::Duration};
use tokio::{runtime, task::block_in_place, time::timeout};
use tracing::{error, field::display, instrument, Span};

#[cfg(test)]
#[async_trait]
#[mockall::automock]
trait Child {
    async fn wait(&mut self) -> io::Result<String>;
    async fn kill(&mut self) -> io::Result<()>;
}

/// An [`Io`] interface for a remote process.
#[derive(Debug)]
pub struct Process {
    #[cfg(test)]
    pipe: Pipe<tokio::io::DuplexStream, tokio::io::DuplexStream>,

    #[cfg(not(test))]
    pipe: Pipe<tokio::process::ChildStdin, tokio::process::ChildStdout>,

    #[cfg(test)]
    child: MockChild,

    #[cfg(not(test))]
    child: tokio::process::Child,
}

impl Process {
    #[cfg(test)]
    const TIMEOUT: Duration = Duration::ZERO;

    #[cfg(not(test))]
    const TIMEOUT: Duration = Duration::from_millis(1000);

    /// Spawns a remote process.
    #[instrument(level = "trace", err)]
    pub fn spawn(path: &str) -> io::Result<Self> {
        #[cfg(test)]
        {
            Ok(Process {
                pipe: tokio::io::duplex(1).into(),
                child: MockChild::new(),
            })
        }

        #[cfg(not(test))]
        {
            let mut child = tokio::process::Command::new(path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()?;

            let pipe = Option::zip(child.stdin.take(), child.stdout.take()).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::Other,
                    Anyhow::msg("failed to open the remote process' stdio"),
                )
            })?;

            Ok(Process {
                pipe: pipe.into(),
                child,
            })
        }
    }
}

/// Flushes the outbound buffer and waits for the remote process to exit.
impl Drop for Process {
    #[instrument(level = "trace", skip(self), fields(status))]
    fn drop(&mut self) {
        let result: Result<_, Anyhow> = block_in_place(|| {
            runtime::Handle::try_current()?.block_on(async {
                self.flush().await?;
                match timeout(Self::TIMEOUT, self.child.wait()).await {
                    Ok(status) => Ok(status?),
                    Err(_) => {
                        self.child.kill().await?;
                        bail!(
                            "timed out after {}s waiting for process to exit",
                            Self::TIMEOUT.as_secs()
                        );
                    }
                }
            })
        });

        match result.context("failed to gracefully terminate the remote process") {
            Err(e) => error!("{:?}", e),
            Ok(s) => {
                Span::current().record("status", display(s));
            }
        }
    }
}

#[async_trait]
impl Io for Process {
    async fn recv(&mut self) -> io::Result<String> {
        self.pipe.recv().await
    }

    async fn send(&mut self, msg: &str) -> io::Result<()> {
        self.pipe.send(msg).await
    }

    async fn flush(&mut self) -> io::Result<()> {
        self.pipe.flush().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::ready;
    use test_strategy::proptest;
    use tokio::time::sleep;

    #[proptest]
    fn drop_gracefully_terminates_child_process(status: String) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let mut process = Process::spawn("")?;

        process
            .child
            .expect_wait()
            .return_once(move || Box::pin(ready(Ok(status))));

        process
            .child
            .expect_kill()
            .return_once(move || Box::pin(ready(Ok(()))));

        rt.block_on(async move {
            drop(process);
        })
    }

    #[proptest]
    fn drop_kills_child_process_if_it_does_not_exit_gracefully(status: String) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let mut process = Process::spawn("")?;

        process.child.expect_wait().return_once(move || {
            Box::pin(async move {
                sleep(Duration::from_secs(1)).await;
                Ok(status)
            })
        });

        process
            .child
            .expect_kill()
            .once()
            .return_once(move || Box::pin(ready(Ok(()))));

        rt.block_on(async move {
            drop(process);
        })
    }

    #[proptest]
    fn drop_recovers_from_errors(a: io::Error, b: io::Error) {
        let rt = runtime::Builder::new_multi_thread().enable_time().build()?;

        let mut process = Process::spawn("")?;

        process
            .child
            .expect_wait()
            .return_once(move || Box::pin(ready(Err(a))));

        process
            .child
            .expect_kill()
            .return_once(move || Box::pin(ready(Err(b))));

        rt.block_on(async move {
            drop(process);
        })
    }

    #[proptest]
    fn drop_recovers_from_missing_runtime() {
        drop(Process::spawn("")?);
    }
}
