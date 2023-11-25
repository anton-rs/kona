<h1 align="center">
<img src="./assets/banner.png" alt="Kona" width="100%" align="center">
</h1>

<h4 align="center">
    A suite of libraries and build pipelines for developing verifiable Rust programs targeting Fault Proof VMs.
</h4>

<p align="center">
  <a href="https://github.com/anton-rs/kona/actions/workflows/ci.yaml">
    <img src="https://github.com/anton-rs/kona/actions/workflows/ci.yaml/badge.svg?label=ci" alt="Ci">
  </a>
  <img src="https://img.shields.io/badge/License-MIT-green.svg?label=license" alt="License">
  <a href="https://github.com/ethereum-optimism/monorepo"><img src="https://img.shields.io/badge/OP%20Stack-monorepo-red" alt="OP Stack"></a>
  <a href="https://t.me/+2yfSX0YikWMxNTRh"><img src="https://img.shields.io/badge/Telegram-x?logo=telegram&label=anton-rs%20contributors"></a>
</p>

<p align="center">
  <a href="#whats-a-cannon">What's Kona?</a> •
  <a href="#overview">Overview</a> •
  <a href="#credits">Credits</a> •
  <a href="#book">Book</a> •
  <a href="#contributing">Contributing</a>
</p>

## What's Kona?

Kona is a suite of libraries and build pipelines for developing verifiable Rust programs targeting Fault Proof VMs. Currently, Kona seeks to support the following targets:
* [`cannon`][cannon] & [`cannon-rs`][cannon-rs]: A `MIPS32rel1` based Fault Proof VM.
* [`asterisc`][asterisc]: A `RISC-V` based Fault Proof VM.

This repository also contains an implementation of the [`op-program` specification][fpp-specs] in Rust, which is used to validate a claim about the state of an [OP Stack rollup][op-stack] on L1 Ethereum.

## Overview

*todo*

```
crates
├── `common-client`: A suite of utilities for developing `client` programs to be ran on top of Fault Proof VMs.
└── `common-host`: A suite of utilities for developing `host` programs.
```

## Credits

`kona` is inspired by the work of several other teams, namely [OP Labs][op-labs] and other contributors' work on the [`op-program`][op-program] and [BadBoiLabs][bad-boi-labs].

## Book

The [book][book] contains a more in-depth overview of the project, tutorials for getting started with building your own programs, and a reference for the libraries and tools provided by Kona.

## Contributing

*TODO - write `CONTRIBUTING.md`*

[op-stack]: https://github.com/ethereum-optimism/optimism
[op-program]: https://github.com/ethereum-optimism/optimism/tree/develop/op-program
[cannon]: https://github.com/ethereum-optimism/optimism/tree/develop/cannon
[cannon-rs]: https://github.com/anton-rs/cannon-rs
[asterisc]: https://github.com/protolambda/asterisc
[fpp-specs]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#fault-proof-program

[book]: https://anton-rs.github.io/kona/

[op-labs]: https://github.com/ethereum-optimism
[bad-boi-labs]: https://github.com/BadBoiLabs
