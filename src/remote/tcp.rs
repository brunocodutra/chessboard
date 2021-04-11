use crate::Remote;
use anyhow::Context;
use async_trait::async_trait;
use derive_more::{DebugCustom, Display, Error, From};
use futures::io::{BufReader, BufWriter, Lines};
use futures::{AsyncBufReadExt, AsyncWriteExt, StreamExt};
use smol::block_on;
use smol::io::{Error as IoError, ErrorKind as IoErrorKind};
use smol::net::{AsyncToSocketAddrs, TcpStream};
use std::fmt::{Debug, Display};
use std::net::SocketAddr;
use tracing::{error, instrument};

/// The reason why connecting to remote TCP server failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "failed to connect to remote TCP server")]
pub struct TcpConnectionError(#[from(forward)] IoError);

/// The reason why writing to or reading from the tcp stream failed.
#[derive(Debug, Display, Error, From)]
#[display(fmt = "the remote TCP connection was interrupted")]
pub struct TcpIoError(#[from(forward)] IoError);

/// An implementation of trait [`Remote`] as a tcp stream.
#[derive(DebugCustom)]
#[debug(fmt = "Tcp({})", address)]
pub struct Tcp {
    address: SocketAddr,
    reader: Lines<BufReader<TcpStream>>,
    writer: BufWriter<TcpStream>,
}

impl Tcp {
    /// Connects to a remote TCP server.
    #[instrument(level = "trace", err)]
    pub async fn connect<A>(address: A) -> Result<Self, TcpConnectionError>
    where
        A: AsyncToSocketAddrs + Debug,
    {
        let socket = TcpStream::connect(address).await?;
        let address = socket.peer_addr()?;

        Ok(Tcp {
            address,
            reader: BufReader::new(socket.clone()).lines(),
            writer: BufWriter::new(socket),
        })
    }
}

/// Flushes the outbound buffer.
impl Drop for Tcp {
    #[instrument(level = "trace")]
    fn drop(&mut self) {
        if let Err(e) = block_on(self.writer.flush()).context("failed to flush the buffer") {
            error!("{:?}", e);
        }
    }
}

#[async_trait]
impl Remote for Tcp {
    type Error = TcpIoError;

    #[instrument(level = "trace", err)]
    async fn recv(&mut self) -> Result<String, Self::Error> {
        use IoErrorKind::UnexpectedEof;
        Ok(self.reader.next().await.ok_or(UnexpectedEof)??)
    }

    #[instrument(level = "trace", skip(item), err, fields(%item))]
    async fn send<D: Display + Send + 'static>(&mut self, item: D) -> Result<(), Self::Error> {
        let line = format!("{}\n", item);
        Ok(self.writer.write_all(line.as_bytes()).await?)
    }

    #[instrument(level = "trace", err)]
    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(self.writer.flush().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Error as Anyhow};
    use futures::{join, AsyncReadExt};
    use port_check::free_local_port;
    use proptest::{collection::vec, prelude::*};
    use smol::net::TcpListener;

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
