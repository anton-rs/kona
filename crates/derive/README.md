# `kona-derive`

> **Notice**: This crate is a WIP.

A `no_std` compatible implementation of the OP Stack's [derivation pipeline][derive].

[derive]: (https://specs.optimism.io/protocol/derivation.html#l2-chain-derivation-specification).

## Features

The most up-to-date feature list will be available on the [docs.rs `Feature Flags` tab][ff] of the `kona-derive` crate.

Some features include the following.
- `serde`: Serialization and Deserialization support for `kona-derive` types.
- `k256`: [secp256k1][k] public key recovery support.
- `online`: Exposes an [alloy-provider][ap] powered data source using "online" HTTP requests.

By default, `kona-derive` enables features `serde` and `k256`.

Key recovery using the [secp256k1][k] curve sits behind a `k256` feature flag so that when compiled in `offline` mode,
secp recovery can fall through to the fpp host, accelerating key recovery. This was necessary since invalid instructions
were found when compiling `k256` recovery down to a bare-metal MIPS target. Since public key recovery requires elliptic
curve pairings, `k256` fall-through host recovery should drastically accelerate derivation on the FPVM.

[k]: https://en.bitcoin.it/wiki/Secp256k1 
[ap]: https://docs.rs/crate/alloy-providers/latest
[ff]: https://docs.rs/crate/kona-derive/latest/features
