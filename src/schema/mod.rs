mod peer;
mod tracker;

pub struct InfoHash([u8; 20]);
pub struct PeerId([u8; 20]);

pub struct MetainfoFile {
    info: Info,
    announce: String,
    info_hash: InfoHash,
}

pub enum Info {
    SingleFile {
        piece_length: u64,
        pieces: Vec<[u8; 20]>,
        name: String,
        length: u64,
    },
    MultiFile {
        piece_length: u64,
        pieces: Vec<[u8; 20]>,
        name: String,
        files: Vec<File>,
    },
}

pub struct File {
    length: u64,
    path: Vec<String>,
}
