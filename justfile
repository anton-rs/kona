set positional-arguments

# default recipe to display help information
default:
  @just --list

# Test for the native target
test *args='':
  cargo nextest run --workspace --all $@

# Lint the workspace for all available targets
lint: lint-native lint-cannon lint-asterisc

# Fixes the formatting of the workspace
fmt-native-fix:
  cargo +nightly fmt --all

# Check the formatting of the workspace
fmt-native-check:
  cargo +nightly fmt --all -- --check

# Lint the workspace
lint-native: fmt-native-check
  cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Lint the workspace (mips arch)
lint-cannon:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/cannon-builder:main cargo +nightly clippy --workspace --all --all-features --target /mips-unknown-none.json -Zbuild-std -- -D warnings 

# Lint the workspace (risc-v arch)
lint-asterisc:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/asterisc-builder:main cargo +nightly clippy --workspace --all --all-features --target riscv64gc-unknown-linux-gnu -Zbuild-std -- -D warnings 

# Build the workspace for all available targets
build: build-native build-cannon build-asterisc

# Build for the native target
build-native *args='':
  cargo build --workspace --all $@

# Build for the `cannon` target
build-cannon *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/cannon-builder:main cargo build --workspace --all -Zbuild-std $@

# Build for the `asterisc` target
build-asterisc *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/ethereum-optimism/kona/asterisc-builder:main cargo build --workspace --all -Zbuild-std $@
