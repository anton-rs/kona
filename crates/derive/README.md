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

[k]: https://en.bitcoin.it/wiki/Secp256k1 
[ap]: https://docs.rs/crate/alloy-providers/latest
[ff]: https://docs.rs/crate/kona-derive/latest/features
