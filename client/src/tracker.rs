use std::collections::HashMap;
use std::iter;
use std::net::IpAddr;
use std::time::Duration;
use tokio::sync::mpsc;

use toytorrent_common as common;

pub struct Incoming {
    pub info_hash: common::InfoHash,
    pub event: IncomingEvent,
}

pub enum IncomingEvent {
    AnnounceResponse { response: common::tracker::Response },
    AnnounceError { url: String, error: String },
    ShouldAnnounce,
}

pub struct Outgoing {
    pub announce_url: String,
    pub info_hash: common::InfoHash,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: Option<common::tracker::Event>,
    pub numwant: Option<u64>,
}

pub async fn announce(
    sender: mpsc::Sender<super::Incoming>,
    mut receiver: mpsc::Receiver<Outgoing>,
    peer_id: common::PeerId,
    key: Option<common::PeerKey>,
    ip: Option<IpAddr>,
    port: u16,
) {
    let mut tracker_ids: HashMap<common::InfoHash, Vec<u8>> = HashMap::new();

    let reqwest_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .default_headers(
            iter::once((
                reqwest::header::USER_AGENT,
                reqwest::header::HeaderValue::from_static(super::USER_AGENT),
            ))
            .collect(),
        )
        .build()
        .unwrap();

    while let Some(outgoing) = receiver.recv().await {
        let request = common::tracker::Request {
            info_hash: outgoing.info_hash,
            uploaded: outgoing.uploaded,
            downloaded: outgoing.downloaded,
            left: outgoing.left,
            event: outgoing.event,
            numwant: outgoing.numwant,

            ip,
            key: key.clone(),
            peer_id,
            port,
            trackerid: tracker_ids.get(&outgoing.info_hash).cloned(),

            compact: None,
            supportcrypto: None,
            requirecrypto: None,
            no_peer_id: None,
        };

        match do_announce(&reqwest_client, &outgoing.announce_url, request).await {
            Ok(response) => {
                // If the server responds with a tracker ID, we are expected to include that ID in
                // future requests.
                if let common::tracker::Response::Success(common::tracker::SuccessResponse {
                    tracker_id: Some(tracker_id),
                    ..
                }) = &response
                {
                    tracker_ids.insert(outgoing.info_hash, tracker_id.clone());
                }

                sender
                    .send(
                        Incoming {
                            info_hash: outgoing.info_hash,
                            event: IncomingEvent::AnnounceResponse { response },
                        }
                        .into(),
                    )
                    .await
                    .ok();
            }
            Err(e) => {
                sender
                    .send(
                        Incoming {
                            info_hash: outgoing.info_hash,
                            event: IncomingEvent::AnnounceError {
                                url: outgoing.announce_url,
                                error: e.into_owned(),
                            },
                        }
                        .into(),
                    )
                    .await
                    .ok();
            }
        }
    }
}

async fn do_announce(
    client: &reqwest::Client,
    announce_url: &str,
    request: common::tracker::Request,
) -> Result<common::tracker::Response, common::Error> {
    let url = if announce_url.contains('?') {
        format!("{announce_url}&{}", request.as_query_string())
    } else {
        format!("{announce_url}?{}", request.as_query_string())
    };

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("{e:?}"))?;

    Ok(response.bytes().await.map_err(|e| format!("{e:?}"))?[..].try_into()?)
}
