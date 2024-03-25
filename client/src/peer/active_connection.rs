use std::io;
use std::marker::PhantomData;

use tokio::io::{AsyncReadExt};
use tokio::net::tcp;

use toytorrent_common as common;
use super::{PendingIncoming, PendingOutgoing, Connection, Incoming, IncomingEvent};

#[derive(Debug)]
pub struct Active;

impl Connection<Active> {
    pub fn from_pending_incoming(connection: Connection<PendingIncoming>) -> Self {
        let (read_stream, write_stream) = connection.stream.unwrap().into_split();

        Self {
            sender: connection.sender,
            stream: None,
            read_stream: Some(read_stream),
            write_stream: Some(write_stream),
            addr: connection.addr,
            my_peer_id: connection.my_peer_id,
            status: PhantomData,
        }
    }

    pub fn from_pending_outgoing(connection: Connection<PendingOutgoing>) -> Self {
        let (read_stream, write_stream) = connection.stream.unwrap().into_split();

        Self {
            sender: connection.sender,
            stream: None,
            read_stream: Some(read_stream),
            write_stream: Some(write_stream),
            addr: connection.addr,
            my_peer_id: connection.my_peer_id,
            status: PhantomData,
        }
    }

    fn read_stream(&mut self) -> &mut tcp::OwnedReadHalf {
        self.read_stream.as_mut().unwrap()
    }

    fn write_stream(&mut self) -> &mut tcp::OwnedWriteHalf {
        self.write_stream.as_mut().unwrap()
    }

    async fn listen(&mut self) -> io::Result<()> {
        let mut len_buf = [0u8; 4];
        let mut buf = [0u8; common::peer::PEERMESSAGE_PIECE_MAX_LEN];

        loop {
            self.read_stream().read_exact(&mut len_buf).await?;
            let len = u32::from_be_bytes(len_buf) as usize;

            if len > common::peer::PEERMESSAGE_PIECE_MAX_LEN {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "Received message too long: max length was {} bytes, got {} bytes",
                        common::peer::PEERMESSAGE_PIECE_MAX_LEN,
                        len,
                    ),
                ));
            }

            self.read_stream().read_exact(&mut buf[..len]).await?;

            match common::peer::PeerMessage::try_from(&buf[..len]) {
                Ok(message) => self
                    .sender
                    .send(
                        Incoming {
                            from_socket_addr: self.addr,
                            event: IncomingEvent::Message { message },
                        }
                        .into(),
                    )
                    .await
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
                Err(e) => eprintln!("{:?}", e),
            }
        }
    }

    async fn send(&mut self, message: common::peer::PeerMessage) -> io::Result<usize> {
        message.write_to(&mut self.write_stream()).await
    }
}
