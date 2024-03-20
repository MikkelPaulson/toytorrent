//! A session corresponds to a single metainfo file download. In the event that multiple torrents
//! are being downloaded at a time, multiple sessions will be opened. Each session has many
//! connections with peers.

use std::fs;
use std::path::Path;

use crate::schema;
use super::Args;

pub async fn open(path: &Path, args: Args) -> ! {
    let metainfo_file: schema::metainfo::MetainfoFile =
        fs::read(path).unwrap()[..].try_into().unwrap();

    let peer_id = schema::PeerId::create("tt", "0000");

    let request = schema::tracker::Request::new(
        metainfo_file.info_hash,
        peer_id,
        args.port,
        0,
        0,
        metainfo_file.info.length(),
    );

    do_announce(&metainfo_file.announce, request).await;

    todo!();
}

async fn do_announce(base_url: &str, request: schema::tracker::Request) {
    let url = if base_url.contains('?') {
        format!("{base_url}&{}", request.as_query_string())
    } else {
        format!("{base_url}?{}", request.as_query_string())
    };
}
