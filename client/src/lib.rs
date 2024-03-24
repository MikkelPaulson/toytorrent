mod announce;
mod connection;
mod session;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use clap::Parser;

use toytorrent_common as common;

const PEER_ID_CLIENT: &'static str = "tt";
const PEER_ID_VERSION: &'static str = "0000";
const USER_AGENT: &'static str = "ToyTorrent/0.0";

/// A barebones BitTorrent client
#[derive(Debug, Parser)]
pub struct Args {
    /// The path to the metainfo (.torrent) file
    file: PathBuf,

    /// The port to listen on
    #[arg(short, long, default_value_t = 6881)]
    port: u16,

    /// The IP address to bind
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: IpAddr,
}

#[derive(Debug, Default)]
struct Torrents(HashMap<common::InfoHash, Torrent>);

#[derive(Debug)]
struct Torrent {
    metainfo: common::metainfo::MetainfoFile,
    peers: HashMap<common::PeerId, connection::Peer>,
    peer_connections: HashMap<SocketAddr, common::PeerId>,
}

enum Incoming {
    Announce(announce::Incoming),
    Connection(connection::Incoming),
    IoError(io::Error),
}

impl From<announce::Incoming> for Incoming {
    fn from(input: announce::Incoming) -> Self {
        Self::Announce(input)
    }
}

impl From<connection::Incoming> for Incoming {
    fn from(input: connection::Incoming) -> Self {
        Self::Connection(input)
    }
}

impl From<io::Error> for Incoming {
    fn from(input: io::Error) -> Self {
        Self::IoError(input)
    }
}

pub async fn run(args: Args) {
    let metainfo: common::metainfo::MetainfoFile =
        fs::read(&args.file).unwrap().as_slice().try_into().unwrap();

    let mut torrents: Torrents = Torrents::default();
    torrents.0.insert(
        *metainfo.info_hash(),
        Torrent {
            metainfo,
            peers: HashMap::new(),
            peer_connections: HashMap::new(),
        },
    );

    let mut connections: HashMap<SocketAddr, connection::Peer> = HashMap::new();
    let mut pending_connections: HashMap<SocketAddr, connection::Connection> = HashMap::new();

    let peer_id = common::PeerId::create("tt", "0000");
    let (incoming_sender, incoming_receiver) = mpsc::channel::<Incoming>();

    let listener = TcpListener::bind(SocketAddr::new(args.bind, args.port))
        .expect("Unable to bind to IP and port");

    thread::spawn(move || connection::listen_for_connections(peer_id, listener, incoming_sender));

    while let Ok(message) = incoming_receiver.recv() {
        match message {
            Incoming::Connection(connection::Incoming {
                from_socket_addr,
                event,
            }) => match event {
                connection::IncomingEvent::Opened { sender, thread } => {
                    pending_connections.insert(
                        from_socket_addr,
                        connection::Connection::new(sender, thread),
                    );
                }
                connection::IncomingEvent::HandshakeInfoHash { info_hash } => {
                    if let Some(connection) = pending_connections.get(&from_socket_addr) {
                        connection
                            .sender
                            .send(connection::Outgoing::Signal(
                                if torrents.0.contains_key(&info_hash) {
                                    connection::Signal::InfoHashOk
                                } else {
                                    connection::Signal::Close
                                },
                            ))
                            .unwrap_or_else(|e| {
                                eprintln!("{:21} !! Unable to send: {:?}", from_socket_addr, e)
                            });
                    } else {
                        eprintln!("{:21} !! Unexpected {:?}", from_socket_addr, event);
                    }
                }
                connection::IncomingEvent::Connected { info_hash, peer_id } => {
                    if let Some(connection) = pending_connections.remove(&from_socket_addr) {
                        connections
                            .insert(from_socket_addr, connection::Peer::new(peer_id, connection));
                    }
                }
                connection::IncomingEvent::Message { message } => todo!(),
                connection::IncomingEvent::Closed => todo!(),
            },
            Incoming::Announce(announce::Incoming { info_hash, event }) => (),
            Incoming::IoError(e) => println!("{:?}", e),
        }
    }
}
