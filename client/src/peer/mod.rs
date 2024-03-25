//! Handles the protocol-level communication with peers.
mod incoming_connection;
mod outgoing_connection;
mod active_connection;

use std::net::SocketAddr;
use std::marker::PhantomData;

use tokio::net::tcp;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};

pub use incoming_connection::PendingIncoming;
pub use outgoing_connection::PendingOutgoing;
pub use active_connection::Active;

use toytorrent_common as common;

#[derive(Debug)]
#[must_use]
pub struct Peer {
    pub peer_id: common::PeerId,
    pub info_hash: common::InfoHash,
    pub connection: Connection<Active>,
    pub am_choking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    pub peer_interested: bool,
    pub bitfield: Vec<u8>,
    pub am_requesting: Vec<common::BlockRef>,
    pub peer_requesting: Vec<common::BlockRef>,
}

#[derive(Debug)]
#[must_use]
pub struct Connection<Status = PendingIncoming> {
    pub sender: mpsc::Sender<super::Incoming>,
    pub addr: SocketAddr,

    stream: Option<TcpStream>,
    read_stream: Option<tcp::OwnedReadHalf>,
    write_stream: Option<tcp::OwnedWriteHalf>,
    my_peer_id: common::PeerId,

    status: PhantomData<Status>,
}

#[derive(Debug)]
pub struct Incoming {
    pub from_socket_addr: SocketAddr,
    pub event: IncomingEvent,
}

#[derive(Debug)]
pub enum IncomingEvent {
    Message {
        message: common::peer::PeerMessage,
    },
    HandshakeInfoHash {
        info_hash: common::InfoHash,
        is_valid_sender: oneshot::Sender<bool>,
    },
    Connected {
        peer: Peer,
    },
    Closed,
}

impl Peer {
    pub fn new(
        peer_id: common::PeerId,
        info_hash: common::InfoHash,
        connection: Connection<Active>,
    ) -> Self {
        Self {
            peer_id,
            info_hash,
            connection,
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
            bitfield: Vec::default(),
            am_requesting: Vec::default(),
            peer_requesting: Vec::default(),
        }
    }

    async fn send(self) {
        self.connection
            .sender
            .clone()
            .send(
                Incoming {
                    from_socket_addr: self.connection.addr,
                    event: IncomingEvent::Connected { peer: self },
                }
                .into(),
            )
            .await
            .unwrap();
    }
}

pub async fn listen(
    my_peer_id: common::PeerId,
    listener: TcpListener,
    sender: mpsc::Sender<super::Incoming>,
) {
    loop {
        if let Err(e) = Connection::<PendingIncoming>::accept(
            listener.accept().await,
            my_peer_id,
            sender.clone(),
        )
        .await
        {
            println!("Error accepting connection: {:?}", e);
        }
    }
}
