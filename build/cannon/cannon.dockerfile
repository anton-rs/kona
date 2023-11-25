# This image and the `mips-unknown-none` target were derived from `BadBoiLabs`'s
# `cannon-rs` project.
#
# https://github.com/BadBoiLabs/Cannon-rs

FROM --platform=linux/amd64 ubuntu:22.04

ENV SHELL=/bin/bash
ENV DEBIAN_FRONTEND noninteractive

# todo: pin `nightly` version
ENV RUST_VERSION nightly

RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
  ca-certificates \
  build-essential \
  curl \
  g++-mips-linux-gnu \
  libc6-dev-mips-cross \
  binutils-mips-linux-gnu \
  llvm \
  clang \
  make \
  cmake \
  git 

# Install Rustup and Rust
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain ${RUST_VERSION} --component rust-src
ENV PATH="/root/.cargo/bin:${PATH}"

# Add the special cannon build target
COPY ./mips-unknown-none.json .

# Set up the env vars to instruct rustc to use the correct compiler and linker
# and to build correctly to support the Cannon processor
ENV CC_mips_unknown_none=mips-linux-gnu-gcc \
  CXX_mips_unknown_none=mips-linux-gnu-g++ \
  CARGO_TARGET_MIPS_UNKNOWN_NONE_LINKER=mips-linux-gnu-gcc \
  RUSTFLAGS="-Clink-arg=-e_start -C llvm-args=-mno-check-zero-division" \
  CARGO_BUILD_TARGET="/mips-unknown-none.json" \
  RUSTUP_TOOLCHAIN=${RUST_VERSION}
