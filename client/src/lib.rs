mod connection;
mod session;

use std::collections::HashMap;
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

struct Torrents(HashMap<common::InfoHash, Torrent>);

struct Torrent {
    metainfo: common::metainfo::MetainfoFile,
    peers: HashMap<common::PeerId, connection::Peer>,
    peer_connections: HashMap<SocketAddr, common::PeerId>,
}

pub async fn run(args: Args) {
    let mut connections: HashMap<SocketAddr, connection::Peer> = HashMap::new();
    let mut pending_connections: HashMap<SocketAddr, connection::Connection> = HashMap::new();

    let peer_id = common::PeerId::create("tt", "0000");
    let (incoming_sender, incoming_receiver) = mpsc::channel::<io::Result<connection::Incoming>>();

    let listener = TcpListener::bind(SocketAddr::new(args.bind, args.port))
        .expect("Unable to bind to IP and port");

    thread::spawn(move || connection::listen_for_connections(peer_id, listener, incoming_sender));

    while let Ok(message) = incoming_receiver.recv() {
        match message {
            Ok(connection::Incoming {
                from_socket_addr,
                event,
            }) => {
                match event {
                    connection::IncomingEvent::Opened { sender, thread } => {
                        pending_connections.insert(from_socket_addr, connection::Connection::new(sender, thread));
                    }
                    connection::IncomingEvent::HandshakeInfoHash { info_hash } => todo!(),
                    connection::IncomingEvent::Connected { info_hash, peer_id } => {
                        if let Some(connection) = pending_connections.remove(&from_socket_addr) {
                            connections.insert(from_socket_addr, connection::Peer::new(peer_id, connection));
                        }
                    }
                    connection::IncomingEvent::Message { message } => todo!(),
                    connection::IncomingEvent::Closed => todo!(),
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
}
