set positional-arguments

# default recipe to display help information
default:
  @just --list

# Test for the native target
test *args='':
  cargo nextest run --workspace --all $@

# Lint the workspace for all available targets
lint: lint-native lint-cannon lint-asterisc

# Lint the workspace
lint-native:
  cargo +nightly fmt --all
  cargo +nightly clippy --workspace --all --all-features --all-targets -- -D warnings

# Lint the workspace (mips arch)
lint-cannon:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    cannon-pipeline:latest cargo +nightly clippy --workspace --all --all-features --target /mips-unknown-none.json -Zbuild-std -- -D warnings 

# Lint the workspace (risc-v arch)
lint-asterisc:
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    asterisc-pipeline:latest cargo +nightly clippy --workspace --all --all-features --target riscv64gc-unknown-linux-gnu -Zbuild-std -- -D warnings 

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
    cannon-pipeline:latest cargo build --workspace --all -Zbuild-std $@

# Build for the `asterisc` target
build-asterisc *args='':
  docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    asterisc-pipeline:latest cargo build --workspace --all -Zbuild-std $@
