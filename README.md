<h1 align="center">
<img src="./assets/banner.png" alt="Kona" width="100%" align="center">
</h1>

<h4 align="center">
    A verifiable implementation of the <a href="https://github.com/ethereum-optimism/optimism">Optimism</a> rollup state transition.
</h4>

<p align="center">
  <a href="https://github.com/ethereum-optimism/kona/actions/workflows/ci.yaml">
    <img src="https://github.com/ethereum-optimism/kona/actions/workflows/ci.yaml/badge.svg?label=ci" alt="CI">
  </a>
  <img src="https://img.shields.io/badge/License-MIT-green.svg?label=license" alt="License">
  <a href="https://ethereum-optimism.github.io/kona">
    <img src="https://img.shields.io/badge/Contributor%20Book-grey?logo=mdBook" alt="Book">
  </a>
  <a href="https://github.com/ethereum-optimism/monorepo"><img src="https://img.shields.io/badge/OP%20Stack-monorepo-red" alt="OP Stack"></a>
  <a href="https://t.me/+2yfSX0YikWMxNTRh"><img src="https://img.shields.io/badge/Telegram-x?logo=telegram&label=anton-rs%20contributors"></a>
</p>

<p align="center">
  <a href="#whats-kona">What's Kona?</a> •
  <a href="#overview">Overview</a> •
  <a href="https://static.optimism.io/kona/CONTRIBUTING.html">Contributing</a> •
  <a href="#book">Book</a> •
  <a href="#credits">Credits</a>
</p>

## What's Kona?

Kona is a [fault proof program][fpp-specs] designed to deterministically execute a rollup state transition in order to
verify an [L2 output root][g-output-root] from the L1 inputs it was [derived from][g-derivation-pipeline].

## Overview

**`kona`**

- [`client`](./bin/client): The bare-metal program that runs on top of a [fault proof VM][g-fault-proof-vm].
- [`host`](./bin/host): The host program that runs natively alongside the FPVM, serving as the [Preimage Oracle][g-preimage-oracle] server.

**Build Pipelines**

- [`cannon`](./build/cannon): Docker image for compiling to the bare-metal `mips-unknown-none` target.
- [`asterisc`](./build/asterisc): Docker image for compiling to the bare-metal `riscv64gc-unknown-none-elf` target.

**`client` / `host` SDK**

- [`common`](./crates/common): A suite of utilities for developing `client` programs to be ran on top of Fault Proof VMs.

## Book

The [book][book] contains a more in-depth overview of the project, contributor guidelines, tutorials for getting started with building your own programs, and a reference for the libraries and tools provided by Kona.

## Credits

`kona` is inspired by the work of several teams, namely [OP Labs][op-labs] and other contributors' work on the [`op-program`][op-program] and [BadBoiLabs][bad-boi-labs]'s work on [Cannon-rs][badboi-cannon-rs].

[op-stack]: https://github.com/ethereum-optimism/optimism
[op-program]: https://github.com/ethereum-optimism/optimism/tree/develop/op-program
[cannon]: https://github.com/ethereum-optimism/optimism/tree/develop/cannon
[cannon-rs]: https://github.com/anton-rs/cannon-rs
[badboi-cannon-rs]: https://github.com/BadBoiLabs/cannon-rs
[asterisc]: https://github.com/protolambda/asterisc
[fpp-specs]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#fault-proof-program
[book]: https://ethereum-optimism.github.io/kona/
[op-labs]: https://github.com/ethereum-optimism
[bad-boi-labs]: https://github.com/BadBoiLabs
[g-output-root]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/glossary.md#l2-output-root
[g-derivation-pipeline]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/derivation.md#l2-chain-derivation-pipeline
[g-fault-proof-vm]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#fault-proof-vm
[g-preimage-oracle]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#pre-image-oracle
