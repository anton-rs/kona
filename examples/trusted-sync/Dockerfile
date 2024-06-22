FROM rust:latest as build

RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools

COPY ./crates/ /kona/crates
COPY ./bin/ /kona/bin
COPY ./examples/ /kona/examples
COPY ./Cargo.toml /kona
COPY ./Cargo.lock /kona
COPY ./rustfmt.toml /kona
COPY ./README.md /kona
WORKDIR /kona

RUN cargo build --release --config net.git-fetch-with-cli=true -p trusted-sync
RUN cp /kona/target/release/trusted-sync /usr/local/bin/trusted-sync
RUN chmod +x ./examples/trusted-sync/run.sh

CMD ["./examples/trusted-sync/run.sh"]
