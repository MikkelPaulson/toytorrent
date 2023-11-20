use super::Peer;

use crate::bencode::BencodeValue;

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

impl From<&Response> for Vec<u8> {
    fn from(input: &Response) -> Self {
        (&BencodeValue::from(input)).into()
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
