mod peer;
mod response;

pub use peer::Peer;
pub use response::Response;

use std::net::IpAddr;

use tide::prelude::Deserialize;

use crate::schema::{InfoHash, PeerId};

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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Started,
    Completed,
    Stopped,
}
