FROM rust:latest AS intermediate

WORKDIR /usr/src/toytorrent

COPY Cargo.* ./
COPY client ./client
COPY common ./common
COPY tracker ./tracker

RUN cargo build --release --package toytorrent-tracker && rm -r ./target/release/build ./target/release/deps

FROM debian:latest

COPY --from=intermediate /usr/src/toytorrent/target/release/toytorrent-tracker /usr/bin/toytorrent-tracker

ENTRYPOINT /usr/bin/toytorrent-tracker
