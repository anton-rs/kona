FROM rust:alpine AS builder

RUN rustup target add x86_64-unknown-linux-musl
# RUN apt-get update && apt-get install -y musl-tools

COPY ./crates/ /kona/crates
COPY ./bin/ /kona/bin
COPY ./examples/ /kona/examples
COPY ./Cargo.toml /kona
COPY ./Cargo.lock /kona
COPY ./rustfmt.toml /kona
COPY ./README.md /kona

WORKDIR /kona

RUN cargo build --release --config net.git-fetch-with-cli=true -p trusted-sync

FROM alpine:3.18

RUN apk add bind-tools jq curl bash git redis libc6-compat libstdc++ openssl ncurses-libs

WORKDIR /kona

COPY --from=builder /kona/examples/trusted-sync/run.sh /kona/run.sh
RUN chmod +x /kona/run.sh

RUN apk update && apk add ca-certificates

COPY --from=builder /kona/target/release/trusted-sync /bin/trusted-sync

CMD ["/kona/run.sh"]
