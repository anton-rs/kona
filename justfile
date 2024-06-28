set positional-arguments
alias t := tests
alias tn := test
alias l := lint
alias ln := lint-native
alias fmt := fmt-native-fix
alias b := build
alias d := docker-build-ts
alias r := docker-run-ts

# default recipe to display help information
default:
  @just --list

# Run all tests
tests: test test-online test-docs

# Test for the native target with all features
test *args='':
  cargo nextest run --workspace --all --all-features $@

# Run online tests
test-online:
  cargo nextest run --workspace --all --features online

# Lint the workspace for all available targets
lint: lint-native lint-cannon lint-asterisc lint-docs

# Fixes the formatting of the workspace
fmt-native-fix:
  cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-native-check:
  cargo +nightly fmt --all -- --check

# Lint the workspace
lint-native: fmt-native-check
  cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Lint the workspace (mips arch). Currently, only the `kona-common` crate is linted for the `cannon` target, as it is the only crate with architecture-specific code.
lint-cannon:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/cannon-builder:main cargo +nightly clippy -p kona-common --all-features --target /mips-unknown-none.json -Zbuild-std -- -D warnings

# Lint the workspace (risc-v arch). Currently, only the `kona-common` crate is linted for the `asterisc` target, as it is the only crate with architecture-specific code.
lint-asterisc:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/asterisc-builder:main cargo +nightly clippy -p kona-common --all-features --target riscv64gc-unknown-linux-gnu -Zbuild-std -- -D warnings

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
  cargo build --workspace --all $@

# Build for the `cannon` target. Any crates that require the stdlib are excluded from the build for this target.
build-cannon *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/cannon-builder:main cargo build --workspace --all -Zbuild-std $@ --exclude kona-host --exclude trusted-sync

# Build for the `asterisc` target. Any crates that require the stdlib are excluded from the build for this target.
build-asterisc *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/asterisc-builder:main cargo build --workspace --all -Zbuild-std $@ --exclude kona-host --exclude trusted-sync

# Build the `trusted-sync` docker image
docker-build-ts *args='':
  docker build -t trusted-sync -f examples/trusted-sync/Dockerfile . $@

# Run the `trusted-sync` docker container
docker-run-ts:
  docker run -it \
    -e L1_RPC_URL=$L1_RPC_URL \
    -e L2_RPC_URL=$L2_RPC_URL \
    -e BEACON_URL=$BEACON_URL \
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
    trusted-sync

