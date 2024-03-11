FROM rust:latest AS intermediate

WORKDIR /usr/src/toytorrent

COPY Cargo.* ./
COPY src ./src

RUN cargo build --release --bin tracker && rm -r ./target/release/build ./target/release/deps

FROM debian:latest

COPY --from=intermediate /usr/src/toytorrent/target/release/tracker /usr/bin/toytorrent-tracker

CMD /usr/bin/toytorrent-tracker
