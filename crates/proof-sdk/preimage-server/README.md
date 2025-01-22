# `kona-preimage-server`

<a href="https://github.com/op-rs/kona/actions/workflows/rust_ci.yaml"><img src="https://github.com/op-rs/kona/actions/workflows/rust_ci.yaml/badge.svg?label=ci" alt="CI"></a>
<a href="https://crates.io/crates/kona-preimage-server"><img src="https://img.shields.io/crates/v/kona-preimage-server.svg?label=kona-preimage-server&labelColor=2a2f35" alt="Kona preimage-server"></a>
<a href="https://github.com/op-rs/kona/blob/main/LICENSE.md"><img src="https://img.shields.io/badge/License-MIT-d1d1f6.svg?label=license&labelColor=2a2f35" alt="License"></a>
<a href="https://img.shields.io/codecov/c/github/op-rs/kona"><img src="https://img.shields.io/codecov/c/github/op-rs/kona" alt="Codecov"></a>

An implementation of the [preimage server](https://specs.optimism.io/fault-proof/index.html#pre-image-oracle) for use
in executing alongside a client program in the guest role.

This implementation supports:
* Online or offline preimage fetching, over a generic key value store.
    * Customizable preimage fetcher, allowing for non-standard hints.
* Default key-value store implementations:
    * In-memory
    * Disk (`rocksdb`)
    * Split (multiplex reads over two implementations of `KeyValueStore`.)
