use std::io;
use std::net::SocketAddr;
use std::marker::PhantomData;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream};
use tokio::sync::{mpsc};

use toytorrent_common as common;
use super::{Active, Connection, Peer};

#[derive(Debug)]
pub struct PendingOutgoing;

impl Connection<PendingOutgoing> {
    async fn connect_to(
        addr: SocketAddr,
        my_peer_id: common::PeerId,
        info_hash: common::InfoHash,
        sender: mpsc::Sender<crate::Incoming>,
    ) -> io::Result<()> {
        let stream = TcpStream::connect(addr).await?;

        let connection = Self {
            sender,
            stream: Some(stream),
            read_stream: None,
            write_stream: None,
            addr,
            my_peer_id,
            status: PhantomData,
        };

        connection.handshake(info_hash).await?.send().await;

        Ok(())
    }

    async fn handshake(mut self, info_hash: common::InfoHash) -> io::Result<Peer> {
        {
            self.stream().write(common::peer::PRELUDE).await?;

            let mut buf = [0; common::peer::PRELUDE.len()];
            self.stream().read_exact(&mut buf).await?;

            if buf != common::peer::PRELUDE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid handshake prelude: {:?}", buf),
                ));
            }
        }

        {
            self.stream().write(info_hash.as_slice()).await?;

            let mut buf = [0; 20];
            self.stream().read_exact(&mut buf).await?;

            if buf != info_hash.as_slice() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Their {:?} does not match our {:?}",
                        common::InfoHash::from(buf),
                        info_hash
                    ),
                ));
            }
        }

        let their_peer_id = {
            let my_peer_id = self.my_peer_id.clone();
            self.stream().write(my_peer_id.as_slice()).await?;

            let mut buf = [0; 20];
            self.stream().read_exact(&mut buf).await?;
            let their_peer_id: common::PeerId = buf.into();

            their_peer_id
        };

        Ok(Peer::new(their_peer_id, info_hash, self.activate()))
    }

    fn stream(&mut self) -> &mut TcpStream {
        self.stream.as_mut().unwrap()
    }

    fn activate(self) -> Connection<Active> {
        Connection::from_pending_outgoing(self)
    }
}
