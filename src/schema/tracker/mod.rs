mod peer;
mod response;

pub use peer::Peer;
pub use response::Response;

use std::net::IpAddr;
use std::str::FromStr;
use std::time::Instant;

use tide::prelude::Deserialize;

use crate::schema::{InfoHash, PeerId};

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
            ip: self.ip.unwrap_or(origin_ip),
            port: self.port,
            uploaded: Some(self.uploaded),
            downloaded: Some(self.downloaded),
            left: Some(self.left),
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

        for clause in input.split('&') {
            if let Some((key, value)) = clause.split_once('=') {
                match key {
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
