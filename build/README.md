# `kona-build`

This directory contains the [`cross`][cross] docker images and custom `rustc` targets used to build verifiable programs targeting various FPVMs.

## Usage

### Building Images

To build the images, run `make` in the root of this directory.

### Compiling Programs

```
docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    cannon-pipeline:latest cargo build --release -Zbuild-std
```

[cross]: https://github.com/cross-rs/cross
