set positional-arguments
alias t := tests
alias la := lint-all
alias l := lint-native
alias f := fmt-native-fix
alias b := build
alias d := docker-build-ts
alias r := docker-run-ts
alias h := hack

# default recipe to display help information
default:
  @just --list

# Run all tests
tests: test test-docs

# Runs `cargo hack check` against the workspace
hack:
  cargo hack check --feature-powerset --no-dev-deps

# Test for the native target with all features
test *args='':
  cargo nextest run --workspace --all --all-features $@

# Lint the workspace for all available targets
lint-all: lint-native lint-cannon lint-asterisc lint-docs

# Fixes the formatting of the workspace
fmt-native-fix:
  cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-native-check:
  cargo +nightly fmt --all -- --check

# Lint the workspace
lint-native: fmt-native-check lint-docs
  cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Lint the workspace (mips arch). Currently, only the `kona-common` crate is linted for the `cannon` target, as it is the only crate with architecture-specific code.
lint-cannon:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/cannon-builder:main cargo +nightly clippy -p kona-common --all-features --target /mips-unknown-none.json -Zbuild-std=core,alloc -- -D warnings

# Lint the workspace (risc-v arch). Currently, only the `kona-common` crate is linted for the `asterisc` target, as it is the only crate with architecture-specific code.
lint-asterisc:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/asterisc-builder:main cargo +nightly clippy -p kona-common --all-features --target riscv64gc-unknown-linux-gnu -Zbuild-std=core,alloc -- -D warnings

# Lint the Rust documentation
lint-docs:
  RUSTDOCFLAGS="-D warnings" cargo doc --all --no-deps --document-private-items 

# Test the Rust documentation
test-docs:
  cargo test --doc --all --locked

# Build the workspace for all available targets
build: build-native build-cannon build-asterisc

# Build for the native target
build-native *args='':
  cargo build --workspace $@

# Build for the `cannon` target. Any crates that require the stdlib are excluded from the build for this target.
build-cannon *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/cannon-builder:main cargo build --workspace -Zbuild-std=core,alloc $@ --exclude kona-host --exclude trusted-sync

# Build for the `asterisc` target. Any crates that require the stdlib are excluded from the build for this target.
build-asterisc *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/asterisc-builder:main cargo build --workspace -Zbuild-std=core,alloc $@ --exclude kona-host --exclude trusted-sync

# Build the `trusted-sync` docker image
docker-build-ts *args='':
  docker build -t trusted-sync -f examples/trusted-sync/Dockerfile . $@

# Run the `trusted-sync` docker container
docker-run-ts:
  docker run -it \
    -e L1_RPC_URL=$L1_RPC_URL \
    -e L2_RPC_URL=$L2_RPC_URL \
    -e BEACON_URL=$BEACON_URL \
    -e METRICS_URL=$METRICS_URL \
    -e START_L2_BLOCK=$START_L2_BLOCK \
    -e START_BLOCKS_FROM_TIP=$START_BLOCKS_FROM_TIP \
    trusted-sync

# Run the `trusted-sync` docker container with Loki logging
docker-run-ts-with-loki:
  docker run -it \
    -e L1_RPC_URL=$L1_RPC_URL \
    -e L2_RPC_URL=$L2_RPC_URL \
    -e BEACON_URL=$BEACON_URL \
    -e LOKI_URL=$LOKI_URL \
    -e METRICS_URL=$METRICS_URL \
    -e START_L2_BLOCK=$START_L2_BLOCK \
    -e START_BLOCKS_FROM_TIP=$START_BLOCKS_FROM_TIP \
    trusted-sync

# Build the `kona-client` prestate artifacts for the latest release.
build-client-prestate-asterisc kona_tag asterisc_tag out='./prestate-artifacts-asterisc':
  #!/bin/bash
  PATH_TO_REPRO_BUILDER=./build/asterisc/asterisc-repro.dockerfile
  OUTPUT_DIR={{out}}

  echo "Building kona-client prestate artifacts for the asterisc target. 🐚 Kona Tag: {{kona_tag}} | 🎇 Asterisc Tag: {{asterisc_tag}}"
  docker build \
    -f $PATH_TO_REPRO_BUILDER \
    --output $OUTPUT_DIR \
    --build-arg CLIENT_TAG={{kona_tag}} \
    --build-arg ASTERISC_TAG={{asterisc_tag}} \
    .
