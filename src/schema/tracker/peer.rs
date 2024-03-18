use std::cmp::{Ord, Ordering, PartialOrd};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, SocketAddr};
use std::time::Instant;

use crate::bencode::BencodeValue;
use crate::schema::{Error, PeerId, PeerKey};

#[derive(Clone, Debug, Eq)]
pub struct Peer {
    pub last_seen: Instant,
    pub peer_id: Option<PeerId>,
    pub addr: SocketAddr,
    pub uploaded: Option<u64>,
    pub downloaded: Option<u64>,
    pub left: Option<u64>,

    pub key: Option<PeerKey>,
    pub supportcrypto: Option<bool>,
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        if let Some(peer_id) = self.peer_id {
            write!(f, "{} ({}) -- ", peer_id, self.addr)?;
        } else {
            write!(f, "{} -- ", self.addr)?;
        }

        if let Some(uploaded) = self.uploaded {
            write!(f, "{} uploaded, ", uploaded)?;
        } else {
            write!(f, "? uploaded, ")?;
        }

        if let Some(downloaded) = self.downloaded {
            write!(f, "{} downloaded, ", downloaded)?;
        } else {
            write!(f, "? downloaded, ")?;
        }

        if let Some(left) = self.left {
            write!(f, "{} left", left)?;
        } else {
            write!(f, "? left")?;
        }

        Ok(())
    }
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.peer_id, other.peer_id) {
            a == b
        } else {
            self.addr == other.addr
        }
    }
}

impl Hash for Peer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if let Some(peer_id) = &self.peer_id {
            peer_id.hash(state);
        }

        if let Some(key) = &self.key {
            key.hash(state);
        } else {
            self.addr.hash(state);
        }
    }
}

impl Ord for Peer {
    fn cmp(&self, other: &Self) -> Ordering {
        if let (Some(a), Some(b)) = (self.peer_id, other.peer_id) {
            a.cmp(&b)
        } else {
            self.addr.cmp(&other.addr)
        }
    }
}

impl PartialOrd for Peer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<BencodeValue<'_>> for Peer {
    type Error = Error;

    fn try_from(input: BencodeValue<'_>) -> Result<Self, Self::Error> {
        let mut input_dict = input.to_dict().ok_or("Peer value must be a dict")?;

        let peer_id = input_dict
            .remove("peer_id".as_bytes())
            .and_then(|benc| benc.to_bytes())
            .and_then(|b| b.as_ref().try_into().ok())
            .ok_or("Missing or invalid peer_id value")?;

        let ip = input_dict
            .remove("ip".as_bytes())
            .and_then(|benc| benc.to_string())
            .and_then(|s| s.parse().ok())
            .ok_or("Missing or invalid IP value")?;

        let port = input_dict
            .remove("port".as_bytes())
            .and_then(|benc| benc.to_u64())
            .and_then(|u| u.try_into().ok())
            .ok_or("Missing or invalid port value")?;

        Ok(Peer {
            last_seen: Instant::now(),
            peer_id: Some(peer_id),
            addr: SocketAddr::new(ip, port),
            uploaded: None,
            downloaded: None,
            left: None,
            key: None,
            supportcrypto: None,
        })
    }
}

impl TryFrom<&[u8]> for Peer {
    type Error = Error;

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        if input.len() != 6 {
            return Err("Short peer values must be 6 bytes long".into());
        }

        let ip = {
            let ip_value: [u8; 4] = input[0..4].try_into().unwrap();
            IpAddr::V4(ip_value.into())
        };

        let port = u16::from_be_bytes(input[4..6].try_into().unwrap());

        Ok(Peer {
            last_seen: Instant::now(),
            peer_id: None,
            addr: SocketAddr::new(ip, port),
            uploaded: None,
            downloaded: None,
            left: None,
            key: None,
            supportcrypto: None,
        })
    }
}

impl TryFrom<Peer> for [u8; 6] {
    type Error = Error;

    fn try_from(input: Peer) -> Result<Self, Self::Error> {
        let mut result = [0; 6];

        let SocketAddr::V4(ipv4_addr) = input.addr else {
            return Err("Only IPv4 values can be encoded with the short syntax".into());
        };

        ipv4_addr
            .ip()
            .octets()
            .into_iter()
            .chain(ipv4_addr.port().to_be_bytes().into_iter())
            .enumerate()
            .for_each(|(i, v)| result[i] = v);

        Ok(result)
    }
}

impl<'a> From<&'a Peer> for BencodeValue<'a> {
    fn from(input: &'a Peer) -> BencodeValue<'a> {
        [
            ("ip", input.addr.ip().to_string().into()),
            ("port", i128::from(input.addr.port()).into()),
        ]
        .into_iter()
        .chain(
            input
                .peer_id
                .iter()
                .map(|peer_id| ("peer id", peer_id.0[..].into())),
        )
        .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn hash_test() {
        use std::collections::HashSet;

        let mut set = HashSet::new();

        assert_eq!(
            true,
            set.insert(Peer {
                last_seen: Instant::now(),
                peer_id: Some([0; 20].into()),
                ip: Ipv4Addr::LOCALHOST.into(),
                port: 65535,
                uploaded: None,
                downloaded: None,
                left: None,
            }),
        );

        assert_eq!(
            false,
            set.insert(Peer {
                last_seen: Instant::now(),
                peer_id: Some([0; 20].into()),
                ip: Ipv4Addr::LOCALHOST.into(),
                port: 65535,
                uploaded: None,
                downloaded: None,
                left: None,
            }),
        );

        assert_eq!(1, set.len());
    }
}
