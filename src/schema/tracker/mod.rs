mod peer;
mod response;

pub use peer::Peer;
pub use response::{FailureResponse, Response, SuccessResponse};

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Instant;

use tide::prelude::Deserialize;

use crate::schema::{InfoHash, PeerId, PeerKey};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Request {
    pub info_hash: InfoHash,
    pub peer_id: PeerId,
    pub ip: Option<IpAddr>,
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: Option<Event>,

    pub numwant: Option<u64>,
    pub key: Option<PeerKey>,
    pub compact: Option<bool>,
    pub supportcrypto: Option<bool>,
    pub no_peer_id: Option<bool>,
    pub trackerid: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Started,
    Completed,
    Stopped,
}

impl Request {
    pub fn as_peer(&self, origin_ip: IpAddr) -> Peer {
        Peer {
            last_seen: Instant::now(),
            peer_id: Some(self.peer_id),
            addr: SocketAddr::new(self.ip.unwrap_or(origin_ip), self.port),
            uploaded: Some(self.uploaded),
            downloaded: Some(self.downloaded),
            left: Some(self.left),
            key: self.key.clone(),
            supportcrypto: self.supportcrypto,
        }
    }
}

impl FromStr for Request {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut info_hash: Option<InfoHash> = None;
        let mut peer_id: Option<PeerId> = None;
        let mut ip: Option<IpAddr> = None;
        let mut port: Option<u16> = None;
        let mut uploaded: Option<u64> = None;
        let mut downloaded: Option<u64> = None;
        let mut left: Option<u64> = None;
        let mut event: Option<Event> = None;
        let mut numwant: Option<u64> = None;
        let mut key: Option<PeerKey> = None;
        let mut compact: Option<bool> = None;
        let mut supportcrypto: Option<bool> = None;
        let mut no_peer_id: Option<bool> = None;
        let mut trackerid: Option<Vec<u8>> = None;

        for clause in input.split('&') {
            if let Some((clause_key, value)) = clause.split_once('=') {
                match clause_key {
                    "info_hash" => info_hash = Some(value.parse()?),
                    "peer_id" => peer_id = Some(value.parse()?),
                    "ip" => ip = Some(value.parse().map_err(|_| "Invalid \"ip\" value")?),
                    "port" => port = Some(value.parse().map_err(|_| "Invalid \"port\" value")?),
                    "uploaded" => {
                        uploaded = Some(value.parse().map_err(|_| "Invalid \"uploaded\" value")?)
                    }
                    "downloaded" => {
                        downloaded =
                            Some(value.parse().map_err(|_| "Invalid \"downloaded\" value")?)
                    }
                    "left" => left = Some(value.parse().map_err(|_| "Invalid \"left\" value")?),
                    "event" => event = Some(value.parse()?),
                    "numwant" => {
                        numwant = Some(value.parse().map_err(|_| "Invalid \"numwant\" value")?)
                    }
                    "key" => key = Some(value.as_bytes().into()),
                    "compact" => compact = Some(value == "1"),
                    "supportcrypto" => supportcrypto = Some(value == "1"),
                    "no_peer_id" => no_peer_id = Some(value == "1"),
                    "trackerid" => trackerid = Some(value.as_bytes().to_vec()),
                    _ => {}
                }
            }
        }

        if let (
            Some(info_hash),
            Some(peer_id),
            Some(port),
            Some(uploaded),
            Some(downloaded),
            Some(left),
        ) = (info_hash, peer_id, port, uploaded, downloaded, left)
        {
            Ok(Request {
                info_hash,
                peer_id,
                ip,
                port,
                uploaded,
                downloaded,
                left,
                event,
                numwant,
                key,
                compact,
                supportcrypto,
                no_peer_id,
                trackerid,
            })
        } else {
            Err("Missing one or more required fields.")
        }
    }
}

impl FromStr for Event {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "started" => Ok(Self::Started),
            "completed" => Ok(Self::Completed),
            "stopped" => Ok(Self::Stopped),
            _ => Err("Unknown event"),
        }
    }
}
