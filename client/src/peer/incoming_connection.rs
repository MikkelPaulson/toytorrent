use std::io;
use std::net::SocketAddr;
use std::marker::PhantomData;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream};
use tokio::sync::{mpsc, oneshot};

use toytorrent_common as common;
use super::{Active, Connection, Incoming, IncomingEvent, Peer};

#[derive(Debug)]
pub struct PendingIncoming;

impl Connection<PendingIncoming> {
    pub async fn accept(
        stream_addr: io::Result<(TcpStream, SocketAddr)>,
        my_peer_id: common::PeerId,
        sender: mpsc::Sender<crate::Incoming>,
    ) -> io::Result<()> {
        let (stream, addr) = stream_addr?;

        let connection = Self {
            sender,
            stream: Some(stream),
            read_stream: None,
            write_stream: None,
            addr,
            my_peer_id,
            status: PhantomData,
        };

        connection.handshake().await?.send().await;

        Ok(())
    }

    async fn handshake(mut self) -> io::Result<Peer> {
        {
            let mut buf = [0; common::peer::PRELUDE.len()];
            self.stream().read_exact(&mut buf).await?;

            if buf != common::peer::PRELUDE {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid handshake prelude: {:?}", buf),
                ));
            }

            self.stream().write(common::peer::PRELUDE).await?;
        }

        {
            let mut buf = [0; common::peer::PRELUDE_RESERVED.len()];
            self.stream().read_exact(&mut buf).await?;

            println!(
                "{}: peer sent prelude {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
                self.addr, buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            );

            self.stream().write(common::peer::PRELUDE_RESERVED).await?;
        }

        let info_hash = {
            let mut buf = [0; 20];
            self.stream().read_exact(&mut buf).await?;
            let info_hash: common::InfoHash = buf.into();

            let (is_valid_sender, is_valid_receiver) = oneshot::channel();

            self.sender
                .send(
                    Incoming {
                        from_socket_addr: self.addr,
                        event: IncomingEvent::HandshakeInfoHash {
                            info_hash,
                            is_valid_sender,
                        },
                    }
                    .into(),
                )
                .await
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            if is_valid_receiver.await != Ok(true) {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Infohash not found: {:?}", info_hash),
                ));
            }

            self.stream().write(info_hash.as_slice()).await?;

            info_hash
        };

        let their_peer_id = {
            let mut buf = [0; 20];
            self.stream().read_exact(&mut buf).await?;
            let their_peer_id: common::PeerId = buf.into();

            let my_peer_id = self.my_peer_id.clone();
            self.stream().write(my_peer_id.as_slice()).await?;

            their_peer_id
        };

        Ok(Peer::new(their_peer_id, info_hash, self.activate()))
    }

    fn stream(&mut self) -> &mut TcpStream {
        self.stream.as_mut().unwrap()
    }

    fn activate(self) -> Connection<Active> {
        Connection::from_pending_incoming(self)
    }
}
