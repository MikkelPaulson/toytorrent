//! Handles the protocol-level communication with peers.

use std::collections::HashMap;
use std::io::prelude::*;
use std::io::BufReader;
use std::net::{SocketAddr, TcpStream};

use crate::schema;

struct Peer {
    peer_id: schema::PeerId,
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
    bitfield: Vec<u8>,
    am_requesting: Vec<schema::BlockRef>,
    peer_requesting: Vec<schema::BlockRef>,
}

impl Peer {
    pub fn new(peer_id: schema::PeerId) -> Self {
        Self {
            peer_id,
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

pub async fn open(mut connection: TcpStream, info_hash: schema::InfoHash, my_peer_id: schema::PeerId) {
    let their_peer_id = handshake(&mut connection, info_hash, my_peer_id, None);

    let mut peer = Peer::new(their_peer_id);

    async {
        read(connection);
    }
    .await;
}

fn handshake(connection: &mut TcpStream, info_hash: schema::InfoHash, my_peer_id: schema::PeerId, expect_peer_id: Option<schema::PeerId>) -> schema::PeerId {
    const PRELUDE: &'static [u8] =  "\u{19}BitTorrent protocol\0\0\0\0\0\0\0\0".as_bytes();
    {
        let mut buf = [0; 28];
        connection.read_exact(&mut buf).unwrap();
        assert_eq!(buf, PRELUDE);
        connection.write(PRELUDE).unwrap();
    }

    {
        let mut buf = [0; 20];
        connection.read_exact(&mut buf).unwrap();
        let other_info_hash: schema::InfoHash = buf.into();
        assert_eq!(info_hash, other_info_hash);
        connection.write(info_hash.as_slice());
    }

    {
        let mut buf = [0; 20];
        connection.read_exact(&mut buf).unwrap();
        let their_peer_id: schema::PeerId = buf.into();
        if let Some(expect_peer_id) = expect_peer_id {
            assert_eq!(expect_peer_id, their_peer_id);
        }
        connection.write(my_peer_id.as_slice());
        their_peer_id
    }
}

fn read<R: std::io::Read>(mut reader: R) -> schema::peer::PeerMessage {
    let mut buf_reader = BufReader::new(reader);

    let len = {
        let mut buffer = [0u8; 4];
        buf_reader.read_exact(&mut buffer).unwrap();
        u32::from_be_bytes(buffer) as usize
    };

    match len {
        ..=schema::peer::PEERMESSAGE_OVERHEAD_MAX_LEN => {
            let mut buffer = [0u8; schema::peer::PEERMESSAGE_OVERHEAD_MAX_LEN];
            buf_reader.read_exact(&mut buffer[0..len]).unwrap();
            schema::peer::PeerMessage::try_from(&buffer[0..len]).unwrap()
        }
        ..=schema::peer::PEERMESSAGE_PIECE_MAX_LEN => {
            let mut buffer = [0u8; schema::peer::PEERMESSAGE_PIECE_MAX_LEN];
            buf_reader.read_exact(&mut buffer[0..len]).unwrap();
            schema::peer::PeerMessage::try_from(&buffer[0..len]).unwrap()
        }
        _ => panic!("Message too large"),
    }
}
