use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

use crate::schema;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Torrents(HashMap<schema::InfoHash, Torrent>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Torrent {
    info_hash: schema::InfoHash,
    pub peers: Peers,
    pub complete: u64,
    pub downloaded: u64,
    pub incomplete: u64,
    pub name: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Peers(BTreeSet<schema::tracker::Peer>);

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
            downloaded: 0,
            incomplete: 0,
            name: None,
        }
    }
}

impl Peers {
    pub fn remove(&mut self, peer: &schema::tracker::Peer) {
        self.0.remove(peer);
    }

    pub fn replace(&mut self, peer: schema::tracker::Peer) {
        self.0.replace(peer);
    }
}

impl Hash for Torrent {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.info_hash.hash(state)
    }
}
