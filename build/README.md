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
    cannon-pipeline:latest cargo build --release -Zbuild-std
```

**asterisc**
```
docker run \
    --rm \
    --platform linux/amd64 \
    -v `pwd`/:/workdir \
    -w="/workdir" \
    asterisc-pipeline:latest cargo build --release -Zbuild-std
```
