use clap::Parser;

use toytorrent::tracker;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let args = tracker::Args::parse();

    tracker::run(args).await
}
