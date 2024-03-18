use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use rand::seq::{IteratorRandom, SliceRandom};

use crate::schema;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Torrents(HashMap<schema::InfoHash, Torrent>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Torrent {
    info_hash: schema::InfoHash,
    pub peers: Peers,
    pub complete: u64,
    pub incomplete: u64,
    pub downloaded: u64,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Peers(HashSet<schema::tracker::Peer>);

impl Torrents {
    pub fn get_or_insert(&mut self, info_hash: schema::InfoHash) -> &mut Torrent {
        self.0
            .entry(info_hash)
            .or_insert_with(|| Torrent::new(info_hash))
    }
}

impl Torrent {
    pub fn new(info_hash: schema::InfoHash) -> Self {
        Self {
            info_hash,
            peers: Peers::default(),
            complete: 0,
            incomplete: 0,
            downloaded: 0,
            name: None,
        }
    }

    pub fn update_counts(&mut self) {
        (self.complete, self.incomplete) = self.peers.complete_incomplete();
    }
}

impl Peers {
    pub fn remove(&mut self, peer: &schema::tracker::Peer) {
        self.0.remove(peer);
    }

    pub fn replace(&mut self, peer: schema::tracker::Peer) {
        self.0.replace(peer);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get_multiple(
        &self,
        count: usize,
        exclude: Option<&schema::tracker::Peer>,
    ) -> Vec<&schema::tracker::Peer> {
        let mut rng = rand::thread_rng();

        let expiry = Self::expiry();

        let mut result = self
            .0
            .iter()
            .filter(|&p| Some(p) != exclude && p.last_seen > expiry)
            .choose_multiple(&mut rng, count);

        result.shuffle(&mut rng);

        result
    }

    fn complete_incomplete(&self) -> (u64, u64) {
        self.0.iter().fold((0, 0), |(complete, incomplete), peer| {
            if peer.left == Some(0) {
                (complete + 1, incomplete)
            } else {
                (complete, incomplete + 1)
            }
        })
    }

    fn expiry() -> Instant {
        Instant::now() - Duration::from_secs(3600)
    }
}

impl Hash for Torrent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.info_hash.hash(state)
    }
}

impl fmt::Display for Torrents {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut keys: Vec<&schema::InfoHash> = self.0.keys().collect();
        keys.sort();

        for torrent in keys.into_iter().filter_map(|key| self.0.get(key)) {
            writeln!(f, "{}", torrent)?;
        }

        Ok(())
    }
}

impl fmt::Display for Torrent {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} -- {} peers, {} complete, {} incomplete, {} downloaded",
            self.info_hash,
            self.peers.len(),
            self.complete,
            self.downloaded,
            self.incomplete,
        )?;

        if !self.peers.is_empty() {
            write!(f, "\n{}", self.peers)?;
        }

        Ok(())
    }
}

impl fmt::Display for Peers {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let mut peer_vec: Vec<&schema::tracker::Peer> = self.0.iter().collect();
        peer_vec.sort();

        for (i, peer) in peer_vec.into_iter().enumerate() {
            if i == 0 {
                write!(f, "{}", peer)?;
            } else {
                write!(f, "\n{}", peer)?;
            }
        }

        Ok(())
    }
}
