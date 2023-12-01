# `kona-build`

This directory contains the [`cross`][cross] docker images and custom `rustc` targets used to build verifiable programs targeting various FPVMs.

## Usage

### Building Images

To build the images, run `make` in the root of this directory.

### Compiling Programs

**cannon**

```
docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    cannon-pipeline:latest cargo build --release -Zbuild-std
```

**asterisc**
```
RUSTFLAGS="-Clink-arg=-e_start -C target-feature=-c" \
    cargo build --target riscv64gc-unknown-none-elf --release
```

[cross]: https://github.com/cross-rs/cross
