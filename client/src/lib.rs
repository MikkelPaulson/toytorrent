mod connection;
mod session;

use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::mpsc::channel;
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

pub async fn run(args: Args) {
    let listener = TcpListener::bind(SocketAddr::new(args.bind, args.port))
        .expect("Unable to bind to IP and port");

    let peer_id = common::PeerId::create("tt", "0000");

    accept_connections(listener);
}

pub fn accept_connections(listener: TcpListener) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                //thread::spawn(move || connection::open(stream));
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}
