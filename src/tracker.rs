use std::net::{IpAddr, SocketAddr};

use clap::Parser;

use super::schema::tracker;

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

    /// The maximum number of peers to return
    #[arg(long, default_value_t = 30)]
    max_response_peers: u32,
}

pub async fn run(args: Args) -> tide::Result<()> {
    let mut app = tide::new();
    app.at("/announce").get(announce);
    app.listen(SocketAddr::from((args.bind, args.port))).await?;

    Ok(())
}

async fn announce(req: tide::Request<()>) -> tide::Result {
    let query: tracker::Request = req.query()?;
    println!("{:?}", req);
    println!("{:?}", query);
    Ok(format!("Blah").into())
}
