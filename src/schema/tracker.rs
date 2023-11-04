use super::{InfoHash, PeerId};

use crate::bencode::BencodeValue;

use std::borrow::Cow;
use std::net::IpAddr;

pub struct Request {
    info_hash: InfoHash,
    peer_id: PeerId,
    ip: Option<IpAddr>,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    event: Option<Event>,
}

pub enum Response {
    Success { interval: u64, peers: Vec<Peer> },
    Failure { failure_reason: String },
}

pub struct Peer {
    peer_id: Option<PeerId>,
    ip: IpAddr,
    port: u16,
}

pub enum Event {
    Started,
    Completed,
    Stopped,
}

impl TryFrom<BencodeValue<'_>> for Response {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let BencodeValue::Dict(mut dict) = input else {
            return Err(());
        };

        if let Some(BencodeValue::Bytes(failure_reason)) = dict.get("failure reason".as_bytes()) {
            Ok(Response::Failure {
                failure_reason: String::from_utf8_lossy(failure_reason).to_string(),
            })
        } else if let (Some(BencodeValue::Integer(interval_value)), Some(peers_value)) = (
            dict.remove("interval".as_bytes()),
            dict.remove("peers".as_bytes()),
        ) {
            let interval = u64::try_from(interval_value).map_err(|_| ())?;

            let peers = match peers_value {
                BencodeValue::List(peer_list) => peer_list
                    .into_iter()
                    .map(|peer| Peer::try_from(peer))
                    .collect::<Result<Vec<_>, _>>()?,
                BencodeValue::Bytes(peer_bytes) => {
                    if peer_bytes.len() % 6 == 0 {
                        peer_bytes
                            .chunks_exact(6)
                            .map(|chunk| Peer::try_from(chunk))
                            .collect::<Result<Vec<Peer>, _>>()?
                    } else {
                        return Err(());
                    }
                }
                _ => return Err(()),
            };

            Ok(Response::Success { interval, peers })
        } else {
            Err(())
        }
    }
}

impl<'a> From<&'a Response> for BencodeValue<'a> {
    fn from(input: &'a Response) -> Self {
        match input {
            Response::Success { interval, peers } => BencodeValue::Dict(
                [
                    (
                        "interval".as_bytes().into(),
                        BencodeValue::Integer((*interval).into()),
                    ),
                    (
                        "peers".as_bytes().into(),
                        BencodeValue::List(peers.iter().map(|peer| peer.into()).collect()),
                    ),
                ]
                .into_iter()
                .collect(),
            )
            .into(),
            Response::Failure { failure_reason } => BencodeValue::Dict(
                [(
                    "failure reason".as_bytes().into(),
                    BencodeValue::Bytes(failure_reason.as_bytes().into()),
                )]
                .into_iter()
                .collect(),
            )
            .into(),
        }
    }
}

impl TryFrom<BencodeValue<'_>> for Peer {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let BencodeValue::Dict(dict) = input else {
            return Err(());
        };

        let (
            Some(BencodeValue::Bytes(peer_id_value)),
            Some(BencodeValue::Bytes(ip_value)),
            Some(BencodeValue::Integer(port_value)),
        ) = (
            dict.get("peer id".as_bytes()),
            dict.get("ip".as_bytes()),
            dict.get("port".as_bytes()),
        )
        else {
            return Err(());
        };

        let peer_id = peer_id_value.as_ref().try_into().map_err(|_| ())?;

        let ip = String::from_utf8(ip_value.to_vec())
            .map_err(|_| ())?
            .parse()
            .map_err(|_| ())?;

        let port = (*port_value).try_into().map_err(|_| ())?;

        Ok(Peer {
            peer_id: Some(peer_id),
            ip,
            port,
        })
    }
}

impl TryFrom<&[u8]> for Peer {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.len() != 6 {
            return Err(());
        }

        let (Ok(ip_value), Ok(port_value)): (Result<[u8; 4], _>, Result<[u8; 2], _>) =
            (input[0..4].try_into(), input[4..6].try_into())
        else {
            return Err(());
        };

        let ip = IpAddr::V4(ip_value.into());
        let port = u16::from_be_bytes(port_value);

        Ok(Peer {
            peer_id: None,
            ip,
            port,
        })
    }
}

impl TryFrom<Peer> for [u8; 6] {
    type Error = ();

    fn try_from(input: Peer) -> Result<Self, Self::Error> {
        let mut result = [0; 6];

        let IpAddr::V4(ipv4_addr) = input.ip else {
            return Err(());
        };

        u32::from(ipv4_addr)
            .to_be_bytes()
            .into_iter()
            .enumerate()
            .for_each(|(i, v)| result[i] = v);

        input
            .port
            .to_be_bytes()
            .into_iter()
            .enumerate()
            .for_each(|(i, v)| result[i + 4] = v);

        Ok(result)
    }
}

impl<'a> From<&'a Peer> for BencodeValue<'a> {
    fn from(input: &'a Peer) -> BencodeValue<'a> {
        BencodeValue::Dict(
            [
                (
                    "ip".as_bytes().into(),
                    BencodeValue::Bytes(Cow::Owned(input.ip.to_string().as_bytes().into())),
                ),
                (
                    "port".as_bytes().into(),
                    BencodeValue::Integer(input.port.into()),
                ),
            ]
            .into_iter()
            .chain(input.peer_id.iter().map(|peer_id| {
                (
                    "peer id".as_bytes().into(),
                    BencodeValue::Bytes(Cow::Owned(peer_id.0.into())),
                )
            }))
            .collect(),
        )
    }
}

impl TryFrom<&[u8]> for PeerId {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        Ok(PeerId(input.try_into().map_err(|_| ())?))
    }
}
