use std::borrow::Cow;
use std::net::IpAddr;

use tide::prelude::Deserialize;

use super::{InfoHash, PeerId};
use crate::bencode::BencodeValue;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Success {
        warning_message: Option<String>,
        interval: u64,
        min_interval: Option<u64>,
        tracker_id: Option<Vec<u8>>,
        complete: Option<u64>,
        incomplete: Option<u64>,
        peers: Vec<Peer>,
    },
    Failure {
        failure_reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Peer {
    peer_id: Option<PeerId>,
    ip: IpAddr,
    port: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Started,
    Completed,
    Stopped,
}

impl TryFrom<&[u8]> for Response {
    type Error = ();

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        BencodeValue::try_from(input)?.try_into()
    }
}

impl TryFrom<BencodeValue<'_>> for Response {
    type Error = ();

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or(())?;

        if let Some(failure_reason) = input_dict
            .remove("failure reason".as_bytes())
            .and_then(BencodeValue::to_string)
        {
            Ok(Response::Failure { failure_reason })
        } else if let (Some(BencodeValue::Integer(interval_value)), Some(peers_value)) = (
            input_dict.remove("interval".as_bytes()),
            input_dict.remove("peers".as_bytes()),
        ) {
            let warning_message = input_dict
                .remove("warning message".as_bytes())
                .and_then(BencodeValue::to_string);

            let interval = u64::try_from(interval_value).map_err(|_| ())?;

            let min_interval = input_dict
                .remove("min interval".as_bytes())
                .and_then(BencodeValue::to_u64);

            let tracker_id = input_dict
                .remove("tracker_id".as_bytes())
                .and_then(BencodeValue::to_bytes)
                .map(|v| v.to_vec());

            let complete = input_dict
                .remove("complete".as_bytes())
                .and_then(BencodeValue::to_u64);

            let incomplete = input_dict
                .remove("incomplete".as_bytes())
                .and_then(BencodeValue::to_u64);

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

            Ok(Response::Success {
                warning_message,
                interval,
                min_interval,
                tracker_id,
                complete,
                incomplete,
                peers,
            })
        } else {
            Err(())
        }
    }
}

impl<'a> From<&'a Response> for BencodeValue<'a> {
    fn from(input: &'a Response) -> Self {
        match input {
            Response::Success {
                warning_message,
                interval,
                min_interval,
                tracker_id,
                complete,
                incomplete,
                peers,
            } => BencodeValue::Dict(
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
                .chain(warning_message.into_iter().map(|s| {
                    (
                        "warning message".as_bytes().into(),
                        BencodeValue::Bytes(s.as_bytes().into()),
                    )
                }))
                .chain(min_interval.into_iter().map(|&i| {
                    (
                        "min interval".as_bytes().into(),
                        BencodeValue::Integer(i.into()),
                    )
                }))
                .chain(tracker_id.into_iter().map(|b| {
                    (
                        "tracker id".as_bytes().into(),
                        BencodeValue::Bytes(b.into()),
                    )
                }))
                .chain(complete.into_iter().map(|&i| {
                    (
                        "complete".as_bytes().into(),
                        BencodeValue::Integer(i.into()),
                    )
                }))
                .chain(incomplete.into_iter().map(|&i| {
                    (
                        "incomplete".as_bytes().into(),
                        BencodeValue::Integer(i.into()),
                    )
                }))
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
        let mut input_dict = input.to_dict().ok_or(())?;

        let (
            Some(BencodeValue::Bytes(peer_id_value)),
            Some(BencodeValue::Bytes(ip_value)),
            Some(BencodeValue::Integer(port_value)),
        ) = (
            input_dict.remove("peer id".as_bytes()),
            input_dict.remove("ip".as_bytes()),
            input_dict.remove("port".as_bytes()),
        )
        else {
            return Err(());
        };

        let peer_id = peer_id_value.as_ref().try_into().map_err(|_| ())?;

        let ip = String::from_utf8(ip_value.to_vec())
            .map_err(|_| ())?
            .parse()
            .map_err(|_| ())?;

        let port = port_value.try_into().map_err(|_| ())?;

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
