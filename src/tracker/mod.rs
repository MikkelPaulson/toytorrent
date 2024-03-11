mod torrent;

use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use clap::Parser;

use crate::schema;

use torrent::Torrents;

static mut TORRENTS: Option<Rc<Mutex<Torrents>>> = None;

/// A barebones BitTorrent tracker
#[derive(Debug, Parser)]
pub struct Args {
    /// The port to listen on
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// The IP address to bind
    #[arg(short, long, default_value = "0.0.0.0")]
    bind: IpAddr,

    /// The interval to instruct clients to announce with
    #[arg(short, long, default_value_t = 600)]
    interval: u32,

    /// If set, the minimum interval to permit clients to announce
    #[arg(long)]
    min_interval: Option<u32>,

    /// The interval after which to consider a client dropped
    #[arg(long, default_value_t = 900)]
    timeout_interval: u32,

    /// The maximum number of peers to return
    #[arg(long, default_value_t = 30)]
    max_response_peers: u32,
}

pub async fn run(args: Args) -> tide::Result<()> {
    unsafe {
        TORRENTS = Some(Rc::new(Mutex::new(Torrents::default())));
    }

    let mut app = tide::new();
    app.at("/announce").get(announce);
    println!("Listening on {}:{}", args.bind, args.port);
    app.listen(SocketAddr::from((args.bind, args.port))).await?;

    Ok(())
}

async fn announce(req: tide::Request<()>) -> tide::Result {
    let request: schema::tracker::Request = req.url().query()
        .ok_or("Missing query")
        .and_then(|s| s.parse())
        .map_err(|_| {
            eprintln!("Missing or invalid query string: {:?}", req.url().query());
            tide::Error::from_str(400, format!("Missing or invalid query string: {:?}", req.url().query()))
        })?;
    println!("{:?}", request);

    let mut torrents = torrents();
    let torrent = torrents
        .get_or_insert(request.info_hash);

    let peer = request.as_peer(
        request
            .ip
            .or_else(|| {
                req
                    .remote()
                    .and_then(|s| s.parse().ok())
            })
            .ok_or_else(|| tide::Error::from_str(500, "No client IP address"))?
    );

    if request.event == Some(schema::tracker::Event::Stopped) {
        torrent.peers.remove(&peer);
    } else {
        torrent.peers.replace(peer);
    }

    if request.event == Some(schema::tracker::Event::Completed) {
        torrent.downloaded += 1;
    }

    let response = schema::tracker::Response::Failure { failure_reason: "Not implemented".to_string() };
    println!("{:?}", response);

    Ok(response.into())
}

fn torrents<'a>() -> MutexGuard<'a, Torrents> {
    unsafe {
        TORRENTS.as_ref().unwrap()
    }.lock().unwrap()
}

impl From<schema::tracker::Response> for tide::Response {
    fn from(input: schema::tracker::Response) -> Self {
        let response_bytes: Vec<u8> = (&input).into();

        tide::Response::builder(200)
            .body(response_bytes)
            .content_type("text/plain")
            .build()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn announce_test() {
        assert_eq!(
            Ok(schema::tracker::Request {
                info_hash: schema::InfoHash::from([
                    0x75, 0x43, 0x9d, 0x5d, 0xe3,
                    0x43, 0x99, 0x9a, 0xb3, 0x77,
                    0xc6, 0x17, 0xc2, 0xc6, 0x47,
                    0x90, 0x29, 0x56, 0xe2, 0x82,
                ]),
                peer_id: schema::PeerId::try_from("-TR4050-mtwvc5ch9psu".as_bytes()).unwrap(),
                ip: None,
                port: 51413,
                uploaded: 0,
                downloaded: 0,
                left: 5037662208,
                event: Some(schema::tracker::Event::Started),
            }),
            "info_hash=uC%9D%5D%E3C%99%9A%B3w%C6%17%C2%C6G%90%29V%E2%82&peer_id=-TR4050-mtwvc5ch9psu&port=51413&uploaded=0&downloaded=0&left=5037662208&numwant=80&key=CE09B16B&compact=1&supportcrypto=1&event=started".parse::<schema::tracker::Request>(),
        );
    }
}