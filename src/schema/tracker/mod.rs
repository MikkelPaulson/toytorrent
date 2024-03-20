mod peer;
mod response;

pub use peer::Peer;
pub use response::{FailureResponse, Response, SuccessResponse};

use std::iter;
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
    pub requirecrypto: Option<bool>,
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
    pub fn new(
        info_hash: InfoHash,
        peer_id: PeerId,
        port: u16,
        uploaded: u64,
        downloaded: u64,
        left: u64,
    ) -> Self {
        Self {
            info_hash,
            peer_id,
            ip: None,
            port,
            uploaded,
            downloaded,
            left,
            event: None,
            numwant: None,
            key: None,
            compact: None,
            supportcrypto: None,
            requirecrypto: None,
            no_peer_id: None,
            trackerid: None,
        }
    }

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
            requirecrypto: self.requirecrypto,
        }
    }

    pub fn as_query_string(&self) -> String {
        let mut query_string = format!(
            "info_hash={info_hash}&peer_id={peer_id}&port={port}&uploaded={uploaded}&downloaded={downloaded}&left={left}",
            info_hash = Self::url_encode(self.info_hash.as_slice()),
            peer_id = Self::url_encode(self.peer_id.as_slice()),
            port = self.port,
            uploaded = self.uploaded,
            downloaded = self.downloaded,
            left = self.left,
        );

        if let Some(ip) = &self.ip {
            query_string.push_str(&format!("&ip={}", ip));
        }

        if let Some(event) = &self.event {
            query_string.push_str("&event=");
            query_string.push_str(event.as_str());
        }

        if let Some(numwant) = &self.numwant {
            query_string.push_str(&format!("&numwant={}", numwant));
        }

        if let Some(key) = &self.key {
            query_string.push_str("&key={}");
            query_string.push_str(&Self::url_encode(key.as_slice()));
        }

        if let Some(compact) = self.compact {
            query_string.push_str("&compact=");
            query_string.push(if compact { '1' } else { '0' });
        }

        if let Some(supportcrypto) = self.supportcrypto {
            query_string.push_str("&supportcrypto=");
            query_string.push(if supportcrypto { '1' } else { '0' });
        }

        if let Some(requirecrypto) = self.requirecrypto {
            query_string.push_str("&requirecrypto=");
            query_string.push(if requirecrypto { '1' } else { '0' });
        }

        if let Some(no_peer_id) = self.no_peer_id {
            query_string.push_str("&no_peer_id=");
            query_string.push(if no_peer_id { '1' } else { '0' });
        }

        if let Some(trackerid) = &self.trackerid {
            query_string.push_str("&trackerid=");
            query_string.push_str(&Self::url_encode(&trackerid[..]));
        }

        query_string
    }

    fn url_encode(slice: &[u8]) -> String {
        slice
            .into_iter()
            .flat_map(|&i| {
                let is_legal = i.is_ascii_alphanumeric();
                iter::once(if is_legal { i as char } else { '%' }).chain(
                    Self::hex_chars(i)
                        .into_iter()
                        .take(if is_legal { 0 } else { 2 }),
                )
            })
            .collect()
    }

    fn hex_chars(input: u8) -> [char; 2] {
        [Self::hex_char(input / 16), Self::hex_char(input % 16)]
    }

    fn hex_char(input: u8) -> char {
        match input {
            0 => '0',
            1 => '1',
            2 => '2',
            3 => '3',
            4 => '4',
            5 => '5',
            6 => '6',
            7 => '7',
            8 => '8',
            9 => '9',
            10 => 'a',
            11 => 'b',
            12 => 'c',
            13 => 'd',
            14 => 'e',
            15 => 'f',
            _ => unreachable!(),
        }
    }
}

impl Event {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Completed => "completed",
            Self::Stopped => "stopped",
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
        let mut requirecrypto: Option<bool> = None;
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
                    "requirecrypto" => requirecrypto = Some(value == "1"),
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
                requirecrypto,
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
