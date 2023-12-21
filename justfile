set positional-arguments

# Lint the workspace
lint:
  cargo +nightly fmt --all && cargo +nightly clippy --all --all-features --all-targets -- -D warnings

# Build the workspace for all available targets
build-all: build-native build-cannon build-asterisc

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
