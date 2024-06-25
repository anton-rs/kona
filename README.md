<h1 align="center">
<img src="./assets/banner.png" alt="Kona" width="100%" align="center">
</h1>

<h4 align="center">
    A verifiable implementation of the <a href="https://github.com/ethereum-optimism/optimism">Optimism</a> rollup state transition.
</h4>

<p align="center">
  <a href="https://github.com/ethereum-optimism/kona/actions/workflows/rust_ci.yaml"><img src="https://github.com/ethereum-optimism/kona/actions/workflows/rust_ci.yaml/badge.svg?label=ci" alt="CI"></a>
  <a href="https://github.com/ethereum-optimism/kona/actions/workflows/fpvm_tests.yaml"><img src="https://github.com/ethereum-optimism/kona/actions/workflows/fpvm_tests.yaml/badge.svg?label=FPVM Tests" alt="FPVM Tests"></a>
  <img src="https://img.shields.io/badge/License-MIT-green.svg?label=license&labelColor=2a2f35" alt="License">
  <a href="https://ethereum-optimism.github.io/kona"><img src="https://img.shields.io/badge/Contributor%20Book-854a15?logo=mdBook&labelColor=2a2f35" alt="Book"></a>
  <a href="https://github.com/ethereum-optimism/monorepo"><img src="https://img.shields.io/badge/OP%20Stack-monorepo-red?labelColor=2a2f35" alt="OP Stack"></a>
</p>

<p align="center">
  <a href="#whats-kona">What's Kona?</a> •
  <a href="#overview">Overview</a> •
  <a href="https://static.optimism.io/kona/CONTRIBUTING.html">Contributing</a> •
  <a href="#credits">Credits</a>
</p>

## What's Kona?

Kona is a suite of portable implementations of the OP Stack rollup state transition, namely the [derivation pipeline][g-derivation-pipeline] and
the block execution logic.

Built on top of these libraries, this repository also features a [fault proof program][fpp-specs] designed to deterministically execute the
rollup state transition in order to verify an [L2 output root][g-output-root] from the L1 inputs it was [derived from][g-derivation-pipeline].

### Development Status

`kona` is currently in active development, and is not yet ready for use in production.

## Overview

**`kona`**

- [`client`](./bin/client): The bare-metal program that runs on top of a [fault proof VM][g-fault-proof-vm].
- [`host`](./bin/host): The host program that runs natively alongside the FPVM, serving as the [Preimage Oracle][g-preimage-oracle] server.

**Build Pipelines**

- [`cannon`](./build/cannon): Docker image for compiling to the bare-metal `mips-unknown-none` target.
- [`asterisc`](./build/asterisc): Docker image for compiling to the bare-metal `riscv64gc-unknown-none-elf` target.

**`client` / `host` SDK**

- [`common`](./crates/common): A suite of utilities for developing `client` programs to be ran on top of Fault Proof VMs.
- [`common-proc`](./crates/common-proc): Proc macro for the `client` program entrypoint.
- [`primitives`](./crates/primitives): Primitive types for use in `kona` crates.
- [`preimage`](./crates/preimage): High level interfaces to the [`PreimageOracle`][fpp-specs] ABI
- [`mpt`](./crates/mpt): Utilities for interacting with the Merkle Patricia Trie in the client program.
- [`executor`](./crates/executor): `no_std` stateless block executor for the [OP Stack][op-stack].
- [`derive`](./crates/derive): `no_std` compatible implementation of the [derivation pipeline][g-derivation-pipeline].
  - [`plasma`](./crates/plasma/): Plasma extension to `kona-derive`

## Book

The [book][book] contains a more in-depth overview of the project, contributor guidelines, tutorials for getting started with building your own programs, and a reference for the libraries and tools provided by Kona.

## Credits

`kona` is inspired by the work of several teams, namely [OP Labs][op-labs] and other contributors' work on the [`op-program`][op-program] and [BadBoiLabs][bad-boi-labs]'s work on [Cannon-rs][badboi-cannon-rs].

[op-stack]: https://github.com/ethereum-optimism/optimism
[op-program]: https://github.com/ethereum-optimism/optimism/tree/develop/op-program
[cannon]: https://github.com/ethereum-optimism/optimism/tree/develop/cannon
[cannon-rs]: https://github.com/anton-rs/cannon-rs
[badboi-cannon-rs]: https://github.com/BadBoiLabs/cannon-rs
[asterisc]: https://github.com/etheruem-optimism/asterisc
[fpp-specs]: https://specs.optimism.io/experimental/fault-proof/index.html
[book]: https://ethereum-optimism.github.io/kona/
[op-labs]: https://github.com/ethereum-optimism
[bad-boi-labs]: https://github.com/BadBoiLabs
[g-output-root]: https://specs.optimism.io/glossary.html#l2-output-root
[g-derivation-pipeline]: https://specs.optimism.io/protocol/derivation.html#l2-chain-derivation-pipeline
[g-fault-proof-vm]: https://specs.optimism.io/experimental/fault-proof/index.html#fault-proof-vm
[g-preimage-oracle]: https://specs.optimism.io/experimental/fault-proof/index.html#pre-image-oracle
