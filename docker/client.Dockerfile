FROM rust:latest AS intermediate

WORKDIR /usr/src/toytorrent

COPY Cargo.* ./
COPY src ./src

RUN cargo build --release --bin client && rm -r ./target/release/build ./target/release/deps

FROM debian:latest

COPY --from=intermediate /usr/src/toytorrent/target/release/client /usr/bin/toytorrent-client

CMD /usr/bin/toytorrent-client
