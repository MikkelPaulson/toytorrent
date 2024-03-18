use crate::schema;

use std::net::IpAddr;

pub async fn announce(
    request: schema::tracker::Request,
    remote_ip: IpAddr,
) -> schema::tracker::Response {
    let mut torrents = super::torrents();
    let torrent = torrents.get_or_insert(request.info_hash);

    let peer = request.as_peer(request.ip.unwrap_or(remote_ip));

    if request.event == Some(schema::tracker::Event::Stopped) {
        torrent.peers.remove(&peer);
    } else {
        torrent.peers.replace(peer.clone());
    }

    if request.event == Some(schema::tracker::Event::Completed) {
        torrent.downloaded += 1;
    }

    torrent.update_counts();

    let peer_count = request
        .numwant
        .and_then(|i| usize::try_from(i).ok())
        .unwrap_or(usize::MAX)
        .clamp(0, 50);

    let peers = torrent.peers.get_multiple(peer_count, Some(&peer)).into_iter().cloned().collect();

    schema::tracker::SuccessResponse {
        warning_message: None,
        interval: 60,
        min_interval: None,
        tracker_id: None,
        complete: Some(torrent.complete),
        incomplete: Some(torrent.incomplete),
        peers,
    }.into()
}
