mod peer;
mod session;
mod tracker;

use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use clap::Parser;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

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
    peers: HashMap<common::PeerId, peer::Peer>,
    peer_connections: HashMap<SocketAddr, common::PeerId>,
}

enum Incoming {
    Tracker(tracker::Incoming),
    Peer(peer::Incoming),
    IoError(io::Error),
}

impl From<tracker::Incoming> for Incoming {
    fn from(input: tracker::Incoming) -> Self {
        Self::Tracker(input)
    }
}

impl From<peer::Incoming> for Incoming {
    fn from(input: peer::Incoming) -> Self {
        Self::Peer(input)
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

    let mut connections: HashMap<SocketAddr, peer::Peer> = HashMap::new();

    let peer_id = common::PeerId::create("tt", "0000");
    let (incoming_sender, mut incoming_receiver) = mpsc::channel::<Incoming>(100);

    let listener = TcpListener::bind(SocketAddr::new(args.bind, args.port))
        .await
        .expect("Unable to bind to IP and port");

    let mut processes = tokio::task::JoinSet::new();

    processes.spawn(peer::listen(peer_id, listener, incoming_sender));

    while let Some(message) = incoming_receiver.recv().await {
        match message {
            Incoming::Peer(peer::Incoming {
                from_socket_addr,
                event,
            }) => match event {
                peer::IncomingEvent::HandshakeInfoHash {
                    info_hash,
                    is_valid_sender,
                } => {
                    is_valid_sender
                        .send(torrents.0.contains_key(&info_hash))
                        .ok();
                }
                peer::IncomingEvent::Connected { peer } => {
                    torrents.0.entry(peer.info_hash).and_modify(|torrent| {
                        torrent.peer_connections.insert(from_socket_addr, peer_id);
                    });
                    connections.insert(peer.connection.addr, peer);
                }
                peer::IncomingEvent::Message { message } => todo!(),
                peer::IncomingEvent::Closed => todo!(),
            },
            Incoming::Tracker(tracker::Incoming { info_hash, event }) => (),
            Incoming::IoError(e) => println!("{:?}", e),
        }
    }
}
