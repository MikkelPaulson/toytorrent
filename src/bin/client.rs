use clap::Parser;

use toytorrent::client;

#[async_std::main]
async fn main() {
    let args = client::Args::parse();

    client::run(args).await;
}
