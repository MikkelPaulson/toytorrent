mod announce;
mod torrent;

use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use clap::Parser;

use toytorrent_common as common;

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
    app.at("/announce").get(announce_route);
    println!("Listening on {}:{}", args.bind, args.port);
    app.listen(SocketAddr::from((args.bind, args.port))).await?;

    Ok(())
}

async fn announce_route(req: tide::Request<()>) -> tide::Result {
    let Some(remote_socket) = req.remote().and_then(|s| s.parse::<SocketAddr>().ok()) else {
        return into_result(common::tracker::FailureResponse {
            failure_reason: "Missing remote address".to_string(),
        });
    };

    println!(
        "{:21} <# {}",
        remote_socket,
        req.url().query().unwrap_or("")
    );
    let request = match req
        .url()
        .query()
        .ok_or("Missing query")
        .and_then(|s| s.parse())
    {
        Ok(r) => r,
        Err(e) => {
            return into_result(common::tracker::FailureResponse {
                failure_reason: e.to_string(),
            });
        }
    };

    println!("{:21} <- {:?}", remote_socket, request);

    let response = announce::announce(request, remote_socket.ip()).await;
    println!("{:21} -> {:?}", remote_socket, response);
    println!(
        "{:21} #> {}\n",
        remote_socket,
        common::BencodeValue::from(&response)
            .encode()
            .into_iter()
            .map(|i| if i.is_ascii() && !i.is_ascii_control() {
                i as char
            } else {
                '.'
            })
            .collect::<String>(),
    );

    println!("{}", torrents());

    into_result(response)
}

fn torrents<'a>() -> MutexGuard<'a, Torrents> {
    unsafe { TORRENTS.as_ref().unwrap() }.lock().unwrap()
}

fn into_result<T: Into<common::tracker::Response>>(response: T) -> tide::Result {
    let tracker_response: common::tracker::Response = response.into();
    let response_bytes: Vec<u8> = (&tracker_response).into();

    Ok(tide::Response::builder(200)
        .body(response_bytes)
        .content_type("text/plain")
        .build())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn announce_test() {
        assert_eq!(
            Ok(common::tracker::Request {
                info_hash: common::InfoHash::from([
                    0x75, 0x43, 0x9d, 0x5d, 0xe3,
                    0x43, 0x99, 0x9a, 0xb3, 0x77,
                    0xc6, 0x17, 0xc2, 0xc6, 0x47,
                    0x90, 0x29, 0x56, 0xe2, 0x82,
                ]),
                peer_id: common::PeerId::try_from("-TR4050-mtwvc5ch9psu".as_bytes()).unwrap(),
                ip: None,
                port: 51413,
                uploaded: 0,
                downloaded: 0,
                left: 5037662208,
                event: Some(common::tracker::Event::Started),

                numwant: Some(80),
                key: Some("CE09B16B".as_bytes().to_vec()),
                compact: Some(true),
                supportcrypto: Some(true),
                no_peer_id: None,
                trackerid: None,
            }),
            "info_hash=uC%9D%5D%E3C%99%9A%B3w%C6%17%C2%C6G%90%29V%E2%82&peer_id=-TR4050-mtwvc5ch9psu&port=51413&uploaded=0&downloaded=0&left=5037662208&numwant=80&key=CE09B16B&compact=1&supportcrypto=1&event=started".parse::<common::tracker::Request>(),
        );
    }
}
