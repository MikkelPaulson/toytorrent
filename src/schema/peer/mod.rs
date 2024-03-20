use std::io;

use super::{BlockRef, InfoHash, PeerId};

#[derive(Clone, Debug)]
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
        block: BlockRef,
    },
    Piece {
        block: BlockRef,
        data: Vec<u8>,
    },
    Cancel {
        block: BlockRef,
    },
    Port {
        port: u16,
    },
}

#[derive(Clone, Debug)]
pub enum ParsedPeerMessage<'a> {
    Complete(PeerMessage, &'a [u8]),
    Incomplete(&'a [u8]),
    Invalid(&'a [u8], &'a [u8]),
}

#[derive(Clone, Debug)]
pub enum PeerMessageError<'a> {
    UnknownId(u8, &'a [u8]),
    BadLength(&'static str, u32, &'a [u8]),
}

const PROTOCOL_NAME: &'static str = "BitTorrent protocol";

const PEERMESSAGE_CHOKE: u8 = 0;
const PEERMESSAGE_UNCHOKE: u8 = 1;
const PEERMESSAGE_INTERESTED: u8 = 2;
const PEERMESSAGE_NOT_INTERESTED: u8 = 3;
const PEERMESSAGE_HAVE: u8 = 4;
const PEERMESSAGE_BITFIELD: u8 = 5;
const PEERMESSAGE_REQUEST: u8 = 6;
const PEERMESSAGE_PIECE: u8 = 7;
const PEERMESSAGE_CANCEL: u8 = 8;
const PEERMESSAGE_PORT: u8 = 9;

const PEERMESSAGE_KEEP_ALIVE_LEN: u32 = 0;
const PEERMESSAGE_CHOKE_LEN: u32 = 1;
const PEERMESSAGE_UNCHOKE_LEN: u32 = 1;
const PEERMESSAGE_INTERESTED_LEN: u32 = 1;
const PEERMESSAGE_NOT_INTERESTED_LEN: u32 = 1;
const PEERMESSAGE_HAVE_LEN: u32 = 5;
const PEERMESSAGE_BITFIELD_MIN_LEN: u32 = 1;
const PEERMESSAGE_REQUEST_LEN: u32 = 13;
const PEERMESSAGE_PIECE_MIN_LEN: u32 = 9;
const PEERMESSAGE_CANCEL_LEN: u32 = 13;
const PEERMESSAGE_PORT_LEN: u32 = 3;

const PIECE_MAX_LEN: u32 = 16 * 1024;
pub const PEERMESSAGE_PIECE_MAX_LEN: usize = (PEERMESSAGE_PIECE_MIN_LEN + PIECE_MAX_LEN) as usize;
pub const PEERMESSAGE_OVERHEAD_MAX_LEN: usize = PEERMESSAGE_REQUEST_LEN as usize;

impl PeerMessage {
    pub fn write_to<W: io::Write>(self, w: &mut W) -> io::Result<usize> {
        let mut l = 0usize;

        match self {
            Self::Handshake { info_hash, peer_id } => {
                l += w.write(&[PROTOCOL_NAME.len() as u8])?;
                l += w.write(PROTOCOL_NAME.as_bytes())?;
                l += w.write(&[0u8; 8])?;
                l += w.write(info_hash.as_slice())?;
                l += w.write(peer_id.as_slice())?;
            }
            Self::KeepAlive => {
                l += w.write(&PEERMESSAGE_KEEP_ALIVE_LEN.to_be_bytes()[..])?;
            }
            Self::Choke => {
                l += w.write(&PEERMESSAGE_CHOKE_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_CHOKE][..])?;
            }
            Self::Unchoke => {
                l += w.write(&PEERMESSAGE_UNCHOKE_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_UNCHOKE][..])?;
            }
            Self::Interested => {
                l += w.write(&PEERMESSAGE_INTERESTED_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_INTERESTED][..])?;
            }
            Self::NotInterested => {
                l += w.write(&PEERMESSAGE_NOT_INTERESTED_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_NOT_INTERESTED][..])?;
            }
            Self::Have { index } => {
                l += w.write(&PEERMESSAGE_HAVE_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_HAVE][..])?;
                l += w.write(&index.to_be_bytes()[..])?;
            }
            Self::Bitfield { bitfield } => {
                l += w.write(
                    &(PEERMESSAGE_BITFIELD_MIN_LEN + bitfield.len() as u32).to_be_bytes()[..],
                )?;
                l += w.write(&[PEERMESSAGE_BITFIELD][..])?;
                l += w.write(&bitfield[..])?;
            }
            Self::Request { block } => {
                l += w.write(&PEERMESSAGE_REQUEST_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_REQUEST][..])?;
                l += w.write(&block.to_be_bytes()[..])?;
            }
            Self::Piece { block, data } => {
                l += w.write(&(PEERMESSAGE_PIECE_MIN_LEN + data.len() as u32).to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_PIECE][..])?;
                l += w.write(&block.to_be_bytes_without_len()[..])?;
                l += w.write(&data[..])?;
            }
            Self::Cancel { block } => {
                l += w.write(&PEERMESSAGE_CANCEL_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_CANCEL][..])?;
                l += w.write(&block.to_be_bytes()[..])?;
            }
            Self::Port { port } => {
                l += w.write(&PEERMESSAGE_PORT_LEN.to_be_bytes()[..])?;
                l += w.write(&[PEERMESSAGE_PORT][..])?;
                l += w.write(&port.to_be_bytes()[..])?;
            }
        }

        Ok(l)
    }
}

impl<'a> From<&'a [u8]> for ParsedPeerMessage<'a> {
    fn from(input: &'a [u8]) -> Self {
        let Some(len) = input
            .get(0..4)
            .map(|b| u32::from_be_bytes(b.try_into().unwrap()) as usize)
        else {
            return ParsedPeerMessage::Incomplete(input);
        };

        if let Some(message) = input.get(1..len) {
            let remainder = input.get(len..).unwrap_or(&[][..]);

            PeerMessage::try_from(message)
                .map(|m| ParsedPeerMessage::Complete(m, remainder))
                .unwrap_or(ParsedPeerMessage::Invalid(&input[..len], remainder))
        } else {
            ParsedPeerMessage::Incomplete(input)
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for PeerMessage {
    type Error = PeerMessageError<'a>;

    fn try_from(input: &'a [u8]) -> Result<Self, Self::Error> {
        if input.is_empty() {
            return Ok(PeerMessage::KeepAlive);
        }

        match (input[0], input.len() as u32) {
            (PEERMESSAGE_CHOKE, PEERMESSAGE_CHOKE_LEN) => Ok(PeerMessage::Choke),
            (PEERMESSAGE_CHOKE, len) => Err(PeerMessageError::BadLength("CHOKE", len, input)),
            (PEERMESSAGE_UNCHOKE, PEERMESSAGE_UNCHOKE_LEN) => Ok(PeerMessage::Unchoke),
            (PEERMESSAGE_UNCHOKE, len) => Err(PeerMessageError::BadLength("UNCHOKE", len, input)),
            (PEERMESSAGE_INTERESTED, PEERMESSAGE_INTERESTED_LEN) => Ok(PeerMessage::Interested),
            (PEERMESSAGE_INTERESTED, len) => {
                Err(PeerMessageError::BadLength("INTERESTED", len, input))
            }
            (PEERMESSAGE_NOT_INTERESTED, PEERMESSAGE_NOT_INTERESTED_LEN) => {
                Ok(PeerMessage::NotInterested)
            }
            (PEERMESSAGE_NOT_INTERESTED, len) => {
                Err(PeerMessageError::BadLength("NOT_INTERESTED", len, input))
            }
            (PEERMESSAGE_HAVE, PEERMESSAGE_HAVE_LEN) => Ok(PeerMessage::Have {
                index: u32::from_be_bytes(input[1..5].try_into().unwrap()),
            }),
            (PEERMESSAGE_HAVE, len) => Err(PeerMessageError::BadLength("HAVE", len, input)),
            (PEERMESSAGE_BITFIELD, len) if len >= PEERMESSAGE_BITFIELD_MIN_LEN => {
                Ok(PeerMessage::Bitfield {
                    bitfield: input[1..].to_vec(),
                })
            }
            (PEERMESSAGE_BITFIELD, len) => Err(PeerMessageError::BadLength("BITFIELD", len, input)),
            (PEERMESSAGE_REQUEST, PEERMESSAGE_REQUEST_LEN) => Ok(PeerMessage::Request {
                block: BlockRef::from_be_bytes(input[1..13].try_into().unwrap()),
            }),
            (PEERMESSAGE_REQUEST, len) => Err(PeerMessageError::BadLength("REQUEST", len, input)),
            (PEERMESSAGE_PIECE, len) if len >= PEERMESSAGE_PIECE_MIN_LEN => {
                Ok(PeerMessage::Piece {
                    block: BlockRef::from_be_bytes_with_len(
                        input[1..9].try_into().unwrap(),
                        len - PEERMESSAGE_PIECE_MIN_LEN,
                    ),
                    data: input[9..].to_vec(),
                })
            }
            (PEERMESSAGE_PIECE, len) => Err(PeerMessageError::BadLength("PIECE", len, input)),
            (PEERMESSAGE_CANCEL, PEERMESSAGE_CANCEL_LEN) => Ok(PeerMessage::Cancel {
                block: BlockRef::from_be_bytes(input[1..13].try_into().unwrap()),
            }),
            (PEERMESSAGE_CANCEL, len) => Err(PeerMessageError::BadLength("CANCEL", len, input)),
            (PEERMESSAGE_PORT, PEERMESSAGE_PORT_LEN) => Ok(PeerMessage::Port {
                port: u16::from_be_bytes(input[1..3].try_into().unwrap()),
            }),
            (PEERMESSAGE_PORT, len) => Err(PeerMessageError::BadLength("PORT", len, input)),
            (i, _) => Err(PeerMessageError::UnknownId(i, input)),
        }
    }
}
