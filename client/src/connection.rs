//! Handles the protocol-level communication with peers.

use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;

use toytorrent_common as common;

#[derive(Debug)]
pub struct Peer {
    pub peer_id: common::PeerId,
    pub connection: Connection,
    pub am_choking: bool,
    pub am_interested: bool,
    pub peer_choking: bool,
    pub peer_interested: bool,
    pub bitfield: Vec<u8>,
    pub am_requesting: Vec<common::BlockRef>,
    pub peer_requesting: Vec<common::BlockRef>,
}

#[derive(Debug)]
pub struct Connection {
    pub established: bool,
    pub sender: mpsc::Sender<Outgoing>,
    pub thread: thread::JoinHandle<io::Result<()>>,
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
    Opened {
        sender: mpsc::Sender<Outgoing>,
        thread: thread::JoinHandle<io::Result<()>>,
    },
    HandshakeInfoHash {
        info_hash: common::InfoHash,
    },
    Connected {
        info_hash: common::InfoHash,
        peer_id: common::PeerId,
    },
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outgoing {
    Message(common::peer::PeerMessage),
    Signal(Signal),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Signal {
    InfoHashOk,
    Close,
}

impl Peer {
    pub fn new(peer_id: common::PeerId, connection: Connection) -> Self {
        Self {
            peer_id,
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
}

impl Connection {
    pub fn new(sender: mpsc::Sender<Outgoing>, thread: thread::JoinHandle<io::Result<()>>) -> Self {
        Self {
            established: false,
            sender,
            thread,
        }
    }
}

pub fn listen_for_connections(
    peer_id: common::PeerId,
    listener: TcpListener,
    sender: mpsc::Sender<io::Result<Incoming>>,
) {
    loop {
        let connection = listener.accept();
        let incoming_sender = sender.clone();

        if let Err(e) = (|| {
            let (stream, addr) = connection?;
            let (outgoing_sender, outgoing_receiver) = mpsc::channel::<Outgoing>();

            let thread = thread::spawn(move || {
                accept_connection(
                    stream.try_clone()?,
                    addr,
                    peer_id,
                    incoming_sender,
                    outgoing_receiver,
                )
            });

            sender
                .send(Ok(Incoming {
                    from_socket_addr: addr,
                    event: IncomingEvent::Opened {
                        sender: outgoing_sender,
                        thread,
                    },
                }))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            io::Result::Ok(())
        })() {
            sender.send(Err(e)).ok();
        }
    }
}

fn accept_connection(
    mut connection: TcpStream,
    addr: SocketAddr,
    peer_id: common::PeerId,
    sender: mpsc::Sender<io::Result<Incoming>>,
    receiver: mpsc::Receiver<Outgoing>,
) -> io::Result<()> {
    let (info_hash, their_peer_id) =
        accept_handshake(&mut connection, addr, peer_id, &sender, &receiver)?;

    sender
        .send(Ok(Incoming {
            from_socket_addr: addr,
            event: IncomingEvent::Connected {
                info_hash,
                peer_id: their_peer_id,
            },
        }))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut buf = [0u8; common::peer::PEERMESSAGE_PIECE_MAX_LEN];
    let mut reader = BufReader::new(connection);

    loop {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf)?;
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

        reader.read_exact(&mut buf[..len])?;

        match common::peer::PeerMessage::try_from(&buf[..len]) {
            Ok(message) => sender
                .send(Ok(Incoming {
                    from_socket_addr: addr,
                    event: IncomingEvent::Message { message },
                }))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?,
            Err(e) => eprintln!("{:?}", e),
        }
    }
}

fn accept_handshake(
    connection: &mut TcpStream,
    addr: SocketAddr,
    my_peer_id: common::PeerId,
    sender: &mpsc::Sender<io::Result<Incoming>>,
    receiver: &mpsc::Receiver<Outgoing>,
) -> io::Result<(common::InfoHash, common::PeerId)> {
    {
        let mut buf = [0; common::peer::PRELUDE.len()];
        connection.read_exact(&mut buf)?;

        if buf != common::peer::PRELUDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid handshake prelude: {:?}", buf),
            ));
        }

        connection.write(common::peer::PRELUDE)?;
    }

    {
        let mut buf = [0; common::peer::PRELUDE_RESERVED.len()];
        connection.read_exact(&mut buf)?;

        println!(
            "{}: peer sent prelude {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x} {:02x}",
            addr, buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        );

        connection.write(common::peer::PRELUDE_RESERVED)?;
    }

    let info_hash = {
        let mut buf = [0; 20];
        connection.read_exact(&mut buf)?;
        let info_hash: common::InfoHash = buf.into();

        sender
            .send(Ok(Incoming {
                from_socket_addr: addr,
                event: IncomingEvent::HandshakeInfoHash { info_hash },
            }))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if receiver.recv() != Ok(Outgoing::Signal(Signal::InfoHashOk)) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Infohash not found: {:?}", info_hash),
            ));
        }

        connection.write(info_hash.as_slice())?;

        info_hash
    };

    let their_peer_id = {
        let mut buf = [0; 20];
        connection.read_exact(&mut buf)?;
        let their_peer_id: common::PeerId = buf.into();

        connection.write(my_peer_id.as_slice())?;

        their_peer_id
    };

    Ok((info_hash, their_peer_id))
}

fn send_handshake(
    connection: &mut TcpStream,
    my_peer_id: common::PeerId,
    info_hash: common::InfoHash,
) -> io::Result<common::PeerId> {
    {
        connection.write(common::peer::PRELUDE)?;

        let mut buf = [0; common::peer::PRELUDE.len()];
        connection.read_exact(&mut buf)?;

        if buf != common::peer::PRELUDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid handshake prelude: {:?}", buf),
            ));
        }
    }

    {
        connection.write(info_hash.as_slice())?;

        let mut buf = [0; 20];
        connection.read_exact(&mut buf)?;

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

    {
        connection.write(my_peer_id.as_slice())?;

        let mut buf = [0; 20];
        connection.read_exact(&mut buf)?;
        let their_peer_id: common::PeerId = buf.into();

        Ok(their_peer_id)
    }
}

fn read<R: Read>(mut reader: R) -> common::peer::PeerMessage {
    let mut buf_reader = BufReader::new(reader);

    let len = {
        let mut buffer = [0u8; 4];
        buf_reader.read_exact(&mut buffer).unwrap();
        u32::from_be_bytes(buffer) as usize
    };

    match len {
        ..=common::peer::PEERMESSAGE_OVERHEAD_MAX_LEN => {
            let mut buffer = [0u8; common::peer::PEERMESSAGE_OVERHEAD_MAX_LEN];
            buf_reader.read_exact(&mut buffer[0..len]).unwrap();
            common::peer::PeerMessage::try_from(&buffer[0..len]).unwrap()
        }
        ..=common::peer::PEERMESSAGE_PIECE_MAX_LEN => {
            let mut buffer = [0u8; common::peer::PEERMESSAGE_PIECE_MAX_LEN];
            buf_reader.read_exact(&mut buffer[0..len]).unwrap();
            common::peer::PeerMessage::try_from(&buffer[0..len]).unwrap()
        }
        _ => panic!("Message too large"),
    }
}
