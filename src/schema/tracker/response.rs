use super::Peer;

use crate::bencode::BencodeValue;
use crate::schema::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    Success(SuccessResponse),
    Failure(FailureResponse),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuccessResponse {
    pub warning_message: Option<String>,
    pub interval: u64,
    pub min_interval: Option<u64>,
    pub tracker_id: Option<Vec<u8>>,
    pub complete: Option<u64>,
    pub incomplete: Option<u64>,
    pub peers: Vec<Peer>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureResponse {
    pub failure_reason: String,
}

impl From<SuccessResponse> for Response {
    fn from(input: SuccessResponse) -> Self {
        Response::Success(input)
    }
}

impl From<FailureResponse> for Response {
    fn from(input: FailureResponse) -> Self {
        Response::Failure(input)
    }
}

impl TryFrom<&[u8]> for Response {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        BencodeValue::decode(input)?.try_into()
    }
}

impl TryFrom<BencodeValue<'_>> for Response {
    type Error = Error;

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or("Response value must be a dict")?;

        if let Some(failure_reason) = input_dict
            .remove("failure reason".as_bytes())
            .and_then(BencodeValue::to_string)
        {
            Ok(Response::Failure(FailureResponse { failure_reason }))
        } else if let (Some(BencodeValue::Integer(interval_value)), Some(peers_value)) = (
            input_dict.remove("interval".as_bytes()),
            input_dict.remove("peers".as_bytes()),
        ) {
            let warning_message = input_dict
                .remove("warning message".as_bytes())
                .and_then(BencodeValue::to_string);

            let interval = u64::try_from(interval_value).map_err(|e| e.to_string())?;

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
                        return Err("Short peer list must be a multiple of 6 bytes long".into());
                    }
                }
                _ => return Err("Peer value must be either a list or byte string".into()),
            };

            Ok(Response::Success(SuccessResponse {
                warning_message,
                interval,
                min_interval,
                tracker_id,
                complete,
                incomplete,
                peers,
            }))
        } else {
            Err("Tracker must respond with either \"interval\" and \"peers\", or \"failure reason\"".into())
        }
    }
}

impl From<&Response> for Vec<u8> {
    fn from(input: &Response) -> Self {
        BencodeValue::from(input).encode()
    }
}

impl<'a> From<&'a Response> for BencodeValue<'a> {
    fn from(input: &'a Response) -> Self {
        match input {
            Response::Success(SuccessResponse {
                warning_message,
                interval,
                min_interval,
                tracker_id,
                complete,
                incomplete,
                peers,
            }) => [
                ("interval", (*interval).into()),
                ("peers", peers.iter().map(BencodeValue::from).collect()),
            ]
            .into_iter()
            .chain(
                warning_message
                    .into_iter()
                    .map(|s| ("warning message", s.as_str().into())),
            )
            .chain(
                min_interval
                    .into_iter()
                    .map(|&i| ("min interval", i.into())),
            )
            .chain(tracker_id.into_iter().map(|b| ("tracker id", b[..].into())))
            .chain(complete.into_iter().map(|&i| ("complete", i.into())))
            .chain(incomplete.into_iter().map(|&i| ("incomplete", i.into())))
            .collect(),
            Response::Failure(FailureResponse { failure_reason }) => {
                [("failure reason", failure_reason.as_str().into())]
                    .into_iter()
                    .collect()
            }
        }
    }
}
