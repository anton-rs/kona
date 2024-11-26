set positional-arguments
alias t := tests
alias la := lint-all
alias l := lint-native
alias lint := lint-native
alias f := fmt-native-fix
alias b := build
alias h := hack

# default recipe to display help information
default:
  @just --list

# Run all tests (excluding online tests)
tests: test test-docs

# Test for the native target with all features. By default, excludes online tests.
test *args="-E '!test(test_online)'":
  cargo nextest run --workspace --all --all-features {{args}}

# Run all online tests
test-online:
  just test "-E 'test(test_online)'"

# Run action tests for the client program on the native target
action-tests test_name='Test_ProgramAction' *args='':
  #!/bin/bash

  just monorepo

  if [ ! -d "monorepo/.devnet" ]; then
    echo "Building devnet allocs for the monorepo"
    (cd monorepo && make devnet-allocs)
  fi

  echo "Building host program for the native target"
  just build-native --bin kona-host --release

  echo "Running action tests for the client program on the native target"
  export KONA_HOST_PATH="{{justfile_directory()}}/target/release/kona-host"
  export KONA_CLIENT_PATH="{{justfile_directory()}}/target/release-client-lto/kona"

  cd monorepo/op-e2e/actions/proofs && \
    gotestsum --format=short-verbose -- -run "{{test_name}}" {{args}} -count=1 ./...

# Clean the action tests directory
clean-actions:
  rm -rf monorepo/

# Lint the workspace for all available targets
lint-all: lint-native lint-cannon lint-asterisc lint-docs

# Runs `cargo hack check` against the workspace
hack:
  cargo hack check --feature-powerset --no-dev-deps

# Fixes the formatting of the workspace
fmt-native-fix:
  cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-native-check:
  cargo +nightly fmt --all -- --check

# Lint the workspace
lint-native: fmt-native-check lint-docs
  cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Lint the workspace (mips arch). Currently, only the `kona-std-fpvm` crate is linted for the `cannon` target, as it is the only crate with architecture-specific code.
lint-cannon:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/cannon-builder:main cargo +nightly clippy -p kona-std-fpvm --all-features --target /mips-unknown-none.json -Zbuild-std=core,alloc -- -D warnings

# Lint the workspace (risc-v arch). Currently, only the `kona-std-fpvm` crate is linted for the `asterisc` target, as it is the only crate with architecture-specific code.
lint-asterisc:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/asterisc-builder:main cargo +nightly clippy -p kona-std-fpvm --all-features --target riscv64gc-unknown-linux-gnu -Zbuild-std=core,alloc -- -D warnings

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
    ghcr.io/anton-rs/kona/cannon-builder:main cargo build --workspace -Zbuild-std=core,alloc $@ --exclude kona-host

# Build for the `asterisc` target. Any crates that require the stdlib are excluded from the build for this target.
build-asterisc *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/asterisc-builder:main cargo build --workspace -Zbuild-std=core,alloc $@ --exclude kona-host

# Build the `kona-client` prestate artifacts for the latest release.
build-client-prestate-asterisc-artifacts kona_tag asterisc_tag out='./prestate-artifacts-asterisc':
  #!/bin/bash
  PATH_TO_REPRO_BUILDER=./build/asterisc/asterisc-repro.dockerfile
  OUTPUT_DIR={{out}}

  echo "Building kona-client prestate artifacts for the asterisc target. 🐚 Kona Tag: {{kona_tag}} | 🎇 Asterisc Tag: {{asterisc_tag}}"
  docker build \
    -f $PATH_TO_REPRO_BUILDER \
    --output $OUTPUT_DIR \
    --build-arg CLIENT_TAG={{kona_tag}} \
    --build-arg ASTERISC_TAG={{asterisc_tag}} \
    --platform linux/amd64 \
    .

# Build the `kona-client` prestate artifacts for the latest release, with an image containing the resulting
# binaries.
build-client-prestate-asterisc-image kona_tag asterisc_tag out='./prestate-artifacts-asterisc':
  #!/bin/bash
  PATH_TO_REPRO_BUILDER=./build/asterisc/asterisc-repro.dockerfile
  OUTPUT_DIR={{out}}

  echo "Building kona-client prestate artifacts for the asterisc target. 🐚 Kona Tag: {{kona_tag}} | 🎇 Asterisc Tag: {{asterisc_tag}}"
  docker build \
    -f $PATH_TO_REPRO_BUILDER \
    -t kona-fpp-asterisc:latest \
    --build-arg CLIENT_TAG={{kona_tag}} \
    --build-arg ASTERISC_TAG={{asterisc_tag}} \
    --platform linux/amd64 \
    .

# Clones and checks out the monorepo at the commit present in `.monorepo`
monorepo:
  ([ ! -d monorepo ] && git clone https://github.com/ethereum-optimism/monorepo) || exit 0
  cd monorepo && git checkout $(cat ../.monorepo)

# Updates the pinned version of the monorepo
update-monorepo:
  [ ! -d monorepo ] && git clone https://github.com/ethereum-optimism/monorepo
  cd monorepo && git rev-parse HEAD > ../.monorepo
