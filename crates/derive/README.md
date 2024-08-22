# `kona-derive`

  <a href="https://github.com/ethereum-optimism/kona/actions/workflows/rust_ci.yaml"><img src="https://github.com/ethereum-optimism/kona/actions/workflows/rust_ci.yaml/badge.svg?label=ci" alt="CI"></a>
    <a href="https://crates.io/crates/kona-derive"><img src="https://img.shields.io/crates/v/kona-derive.svg?label=kona-derive&labelColor=2a2f35" alt="Kona Derive"></a>
  <img src="https://img.shields.io/badge/License-MIT-green.svg?label=license&labelColor=2a2f35" alt="License">

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
