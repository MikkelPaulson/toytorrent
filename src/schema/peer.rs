use std::iter;

use super::{InfoHash, PeerId};

pub enum PeerMessage {
    Handshake {
        info_hash: InfoHash,
        peer_id: PeerId,
    },
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have {
        index: u32,
    },
    Bitfield {
        bitfield: Vec<u8>,
    },
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        piece: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

const PROTOCOL_NAME: &'static str = "BitTorrent protocol";

const PEERMESSAGE_CHOKE: u8 = 0;
const PEERMESSAGE_UNCHOKE: u8 = 1;
const PEERMESSAGE_INTERESTED: u8 = 2;
const PEERMESSAGE_NOTINTERESTED: u8 = 3;
const PEERMESSAGE_HAVE: u8 = 4;
const PEERMESSAGE_BITFIELD: u8 = 5;
const PEERMESSAGE_REQUEST: u8 = 6;
const PEERMESSAGE_PIECE: u8 = 7;
const PEERMESSAGE_CANCEL: u8 = 8;

const PEERMESSAGE_CHOKE_LEN: usize = 1;
const PEERMESSAGE_UNCHOKE_LEN: usize = 1;
const PEERMESSAGE_INTERESTED_LEN: usize = 1;
const PEERMESSAGE_NOTINTERESTED_LEN: usize = 1;
const PEERMESSAGE_HAVE_LEN: usize = 5;
const PEERMESSAGE_BITFIELD_MIN_LEN: usize = 1;
const PEERMESSAGE_REQUEST_LEN: usize = 13;
const PEERMESSAGE_PIECE_MIN_LEN: usize = 9;
const PEERMESSAGE_CANCEL_LEN: usize = 13;

impl From<PeerMessage> for Vec<u8> {
    fn from(input: PeerMessage) -> Self {
        let mut result = [0; 4].to_vec();

        match input {
            PeerMessage::Handshake { info_hash, peer_id } => {
                result.clear();

                result.extend(
                    iter::once(PROTOCOL_NAME.len() as u8)
                        .chain(PROTOCOL_NAME.bytes())
                        .chain(iter::repeat(0).take(8))
                        .chain(info_hash.0.into_iter())
                        .chain(peer_id.0.into_iter()),
                );
            }
            PeerMessage::KeepAlive => {}
            PeerMessage::Choke => result.push(PEERMESSAGE_CHOKE),
            PeerMessage::Unchoke => result.push(PEERMESSAGE_UNCHOKE),
            PeerMessage::Interested => result.push(PEERMESSAGE_INTERESTED),
            PeerMessage::NotInterested => result.push(PEERMESSAGE_NOTINTERESTED),
            PeerMessage::Have { index } => {
                result.extend(iter::once(PEERMESSAGE_HAVE).chain(index.to_be_bytes()))
            }
            PeerMessage::Bitfield { bitfield } => {
                result.extend(iter::once(PEERMESSAGE_BITFIELD).chain(bitfield.into_iter()))
            }
            PeerMessage::Request {
                index,
                begin,
                length,
            } => result.extend(
                iter::once(PEERMESSAGE_REQUEST)
                    .chain(index.to_be_bytes())
                    .chain(begin.to_be_bytes())
                    .chain(length.to_be_bytes()),
            ),
            PeerMessage::Piece {
                index,
                begin,
                piece,
            } => result.extend(
                iter::once(PEERMESSAGE_PIECE)
                    .chain(index.to_be_bytes())
                    .chain(begin.to_be_bytes())
                    .chain(piece.into_iter()),
            ),
            PeerMessage::Cancel {
                index,
                begin,
                length,
            } => result.extend(
                iter::once(PEERMESSAGE_CANCEL)
                    .chain(index.to_be_bytes())
                    .chain(begin.to_be_bytes())
                    .chain(length.to_be_bytes()),
            ),
        }

        result.splice(0..4, ((result.len() - 4) as u32).to_be_bytes().into_iter());
        result
    }
}

impl TryFrom<&[u8]> for PeerMessage {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.is_empty() {
            return Ok(PeerMessage::KeepAlive);
        }

        Ok(match (input[0], input.len()) {
            (PEERMESSAGE_CHOKE, PEERMESSAGE_CHOKE_LEN) => PeerMessage::Choke,
            (PEERMESSAGE_UNCHOKE, PEERMESSAGE_UNCHOKE_LEN) => PeerMessage::Unchoke,
            (PEERMESSAGE_INTERESTED, PEERMESSAGE_INTERESTED_LEN) => PeerMessage::Interested,
            (PEERMESSAGE_NOTINTERESTED, PEERMESSAGE_NOTINTERESTED_LEN) => {
                PeerMessage::NotInterested
            }
            (PEERMESSAGE_HAVE, PEERMESSAGE_HAVE_LEN) => PeerMessage::Have {
                index: u32::from_be_bytes(input[1..5].try_into().unwrap()),
            },
            (PEERMESSAGE_BITFIELD, len) if len >= PEERMESSAGE_BITFIELD_MIN_LEN => {
                PeerMessage::Bitfield {
                    bitfield: input[1..].to_vec(),
                }
            }
            (PEERMESSAGE_REQUEST, PEERMESSAGE_REQUEST_LEN) => PeerMessage::Request {
                index: u32::from_be_bytes(input[1..5].try_into().unwrap()),
                begin: u32::from_be_bytes(input[5..9].try_into().unwrap()),
                length: u32::from_be_bytes(input[9..13].try_into().unwrap()),
            },
            (PEERMESSAGE_PIECE, len) if len >= PEERMESSAGE_PIECE_MIN_LEN => PeerMessage::Piece {
                index: u32::from_be_bytes(input[1..5].try_into().unwrap()),
                begin: u32::from_be_bytes(input[5..9].try_into().unwrap()),
                piece: input[9..].to_vec(),
            },
            (PEERMESSAGE_CANCEL, PEERMESSAGE_CANCEL_LEN) => PeerMessage::Cancel {
                index: u32::from_be_bytes(input[1..5].try_into().unwrap()),
                begin: u32::from_be_bytes(input[5..9].try_into().unwrap()),
                length: u32::from_be_bytes(input[9..13].try_into().unwrap()),
            },
            _ => return Err(()),
        })
    }
}
