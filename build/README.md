# `kona-build`

This directory contains the cross compilation docker images and custom `rustc` targets used to build verifiable programs targeting various FPVMs.

## Usage

### Building Images

To build the images, run `just` in the root of this directory.

### Compiling Programs

**cannon**

```
docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/cannon-builder:main cargo build --release -Zbuild-std
```

**asterisc**

```
docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    ghcr.io/anton-rs/kona/asterisc-builder:main cargo build --release -Zbuild-std
```
