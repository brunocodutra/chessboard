use crate::Remote;
use anyhow::Context;
use async_trait::async_trait;
use derive_more::{Display, Error, From};
use smol::{block_on, io, net::*, prelude::*};
use std::fmt::Display;
use tracing::*;

/// The reason why connecting to remote TCP server failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "failed to connect to remote TCP server")]
pub struct TcpConnectionError(io::Error);

/// The reason why writing to or reading from the tcp stream failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "the remote TCP connection was terminated")]
pub struct TcpIoError(io::Error);

impl From<io::ErrorKind> for TcpIoError {
    fn from(k: io::ErrorKind) -> Self {
        io::Error::from(k).into()
    }
}

/// An implementation of trait [`Remote`] as a tcp stream.
pub struct Tcp {
    reader: io::Lines<io::BufReader<TcpStream>>,
    writer: io::BufWriter<TcpStream>,
}

impl Tcp {
    #[instrument(skip(addrs), err)]
    pub async fn connect<A: AsyncToSocketAddrs>(addrs: A) -> Result<Self, TcpConnectionError> {
        let socket = TcpStream::connect(addrs).await?;
        info!(local_addr = %socket.local_addr()?, peer_addr = %socket.peer_addr()?);

        Ok(Tcp {
            reader: io::BufReader::new(socket.clone()).lines(),
            writer: io::BufWriter::new(socket),
        })
    }
}

/// Flushes the outbound buffer.
impl Drop for Tcp {
    #[instrument(skip(self))]
    fn drop(&mut self) {
        if let Err(e) = block_on(self.flush()).context("failed to flush the buffer") {
            error!("{:?}", e);
        }
    }
}

#[async_trait]
impl Remote for Tcp {
    type Error = TcpIoError;

    #[instrument(skip(self), /*err*/)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        let next = self.reader.next().await;
        let line = next.ok_or(io::ErrorKind::UnexpectedEof)??;
        trace!(%line);
        Ok(line)
    }

    #[instrument(skip(self, msg), /*err*/)]
    async fn send<D: Display + Send + 'static>(&mut self, msg: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", msg);
        trace!(%line);
        self.writer.write_all(line.as_bytes()).await?;
        Ok(())
    }

    #[instrument(skip(self), /*err*/)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Error as Anyhow};
    use futures::join;
    use port_check::free_local_port;
    use proptest::{collection::vec, prelude::*};

    async fn connect() -> Result<(Tcp, TcpStream), Anyhow> {
        let port = free_local_port().context("no free port")?;
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        let (connected, accepted) = join!(Tcp::connect(addr), listener.accept());

        let tcp = connected?;
        let (peer, _) = accepted?;

        Ok((tcp, peer))
    }

    proptest! {
        #[test]
        fn send_appends_new_line_to_message(msg: String) {
            block_on(async {
                let (mut tcp, mut peer) = connect().await.unwrap();

                tcp.send(msg.clone()).await.unwrap();
                drop(tcp);

                let mut received = String::new();
                peer.read_to_string(&mut received).await.unwrap();

                assert_eq!(received, format!("{}\n", msg));
            });
        }

        #[test]
        fn recv_splits_by_new_line(msgs in vec("[^\r\n]*\n", 0..=10)) {
            block_on(async {
                let (mut tcp, mut peer) = connect().await.unwrap();

                peer.write_all(msgs.concat().as_bytes()).await.unwrap();
                peer.flush().await.unwrap();
                drop(peer);

                let mut received = vec![];
                while let Ok(msg) = tcp.recv().await {
                    received.push(format!("{}\n", msg));
                }

                assert_eq!(received, msgs);
            });
        }
    }
}
