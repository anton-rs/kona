################################################################
#                Build Asterisc @ `ASTERISC_TAG`               #
################################################################

FROM ubuntu:22.04 AS asterisc-build
SHELL ["/bin/bash", "-c"]

ARG ASTERISC_TAG

# Install deps
RUN apt-get update && apt-get install -y --no-install-recommends git curl ca-certificates make

ENV GO_VERSION=1.21.1

# Fetch go manually, rather than using a Go base image, so we can copy the installation into the final stage
RUN curl -sL https://go.dev/dl/go$GO_VERSION.linux-amd64.tar.gz -o go$GO_VERSION.linux-amd64.tar.gz && \
  tar -C /usr/local/ -xzf go$GO_VERSION.linux-amd64.tar.gz
ENV GOPATH=/go
ENV PATH=/usr/local/go/bin:$GOPATH/bin:$PATH

# Clone and build Asterisc @ `ASTERISC_TAG`
RUN git clone https://github.com/ethereum-optimism/asterisc && \
  cd asterisc && \
  git checkout $ASTERISC_TAG && \
  make && \
  cp rvgo/bin/asterisc /asterisc-bin

################################################################
#               Build kona-client @ `CLIENT_TAG`               #
################################################################

FROM ghcr.io/op-rs/kona/asterisc-builder@sha256:56c57453ebe09875e96df527d3734d781e987dbdc1e0ce9e813e1e88590036bd AS client-build
SHELL ["/bin/bash", "-c"]

ARG CLIENT_TAG

# Copy the Rust workspace from the host
COPY ./.git ./.git
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./crates ./crates
COPY ./bin ./bin

# Install deps
RUN apt-get update && apt-get install -y --no-install-recommends git

# Build kona-client on the selected tag
RUN git checkout $CLIENT_TAG && \
  cargo build -Zbuild-std=core,alloc --workspace --bin kona --locked --profile release-client-lto --exclude kona-host --exclude kona-derive-alloy && \
  mv ./target/riscv64imac-unknown-none-elf/release-client-lto/kona /kona-client-elf

################################################################
#                Build kona-host @ `CLIENT_TAG`                #
################################################################

FROM ubuntu:22.04 AS host-build
SHELL ["/bin/bash", "-c"]

ARG CLIENT_TAG

# Copy the Rust workspace from the host
COPY ./.git ./.git
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./crates ./crates
COPY ./bin ./bin

# Install deps
RUN apt-get update && apt-get install -y --no-install-recommends \
  build-essential \
  git \
  curl \
  ca-certificates \
  libssl-dev \
  clang \
  pkg-config

# Install rust
ENV RUST_VERSION=1.81.0
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain ${RUST_VERSION} --component rust-src
ENV PATH="/root/.cargo/bin:${PATH}"

# Build kona-host on the selected tag
RUN git checkout $CLIENT_TAG && \
  cargo build --workspace --bin kona-host --release && \
  mv ./target/release/kona-host /kona-host

################################################################
#      Create `prestate.bin.gz` + `prestate-proof.json`        #
################################################################

FROM ubuntu:22.04 AS prestate-build
SHELL ["/bin/bash", "-c"]

# Set env
ENV ASTERISC_BIN_PATH="/asterisc"
ENV CLIENT_BIN_PATH="/kona-client-elf"
ENV PRESTATE_OUT_PATH="/prestate.bin.gz"
ENV PROOF_OUT_PATH="/prestate-proof.json"

# Copy asterisc binary
COPY --from=asterisc-build /asterisc-bin $ASTERISC_BIN_PATH

# Copy kona-client binary
COPY --from=client-build /kona-client-elf $CLIENT_BIN_PATH

# Create `prestate.bin.gz`
RUN $ASTERISC_BIN_PATH load-elf \
  --path=$CLIENT_BIN_PATH \
  --out=$PRESTATE_OUT_PATH

# Create `prestate-proof.json`
RUN $ASTERISC_BIN_PATH run \
  --proof-at "=0" \
  --stop-at "=1" \
  --input $PRESTATE_OUT_PATH \
  --meta ./meta.json \
  --proof-fmt "./%d.json" \
  --output "" && \
  mv 0.json $PROOF_OUT_PATH

################################################################
#                       Export Artifacts                       #
################################################################

FROM ubuntu:22.04 AS export-stage

COPY --from=prestate-build /asterisc .
COPY --from=prestate-build /kona-client-elf .
COPY --from=prestate-build /prestate.bin.gz .
COPY --from=prestate-build /prestate-proof.json .
COPY --from=host-build /kona-host .
