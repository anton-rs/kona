<h1 align="center">
<img src="./assets/banner.png" alt="Kona" width="100%" align="center">
</h1>

<h4 align="center">
    A verifiable implementation of the [Optimism][op-stack] rollup state transition.
</h4>

<p align="center">
  <a href="https://github.com/anton-rs/kona/actions/workflows/ci.yaml">
    <img src="https://github.com/anton-rs/kona/actions/workflows/ci.yaml/badge.svg?label=ci" alt="CI">
  </a>
  <img src="https://img.shields.io/badge/License-MIT-green.svg?label=license" alt="License">
  <a href="https://anton-rs.github.io/kona">
    <img src="https://img.shields.io/badge/Contributor%20Book-grey?logo=mdBook" alt="Book">
  </a>
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

Kona is a [fault proof program] designed to execute a rollup state transition and ultimately verify an [L2 output root][g-output-root] from
L1 inputs, derived through the rollup's [derivation pipeline][g-derivation-pipeline].

## Overview

*TODO - overview after mockup*

```
crates
├── `common`: A suite of utilities for developing `client` programs to be ran on top of Fault Proof VMs.
└── `placeholder`: Placeholder
```

## Book

The [book][book] contains a more in-depth overview of the project, tutorials for getting started with building your own programs, and a reference for the libraries and tools provided by Kona.

## Contributing

*TODO - write `CONTRIBUTING.md`*

## Credits

`kona` is inspired by the work of several teams, namely [OP Labs][op-labs] and other contributors' work on the [`op-program`][op-program] and [BadBoiLabs][bad-boi-labs]'s work on [Cannon-rs][badboi-cannon-rs].

[op-stack]: https://github.com/ethereum-optimism/optimism
[op-program]: https://github.com/ethereum-optimism/optimism/tree/develop/op-program
[cannon]: https://github.com/ethereum-optimism/optimism/tree/develop/cannon
[cannon-rs]: https://github.com/anton-rs/cannon-rs
[badboi-cannon-rs]: https://github.com/BadBoiLabs/cannon-rs
[asterisc]: https://github.com/protolambda/asterisc
[fpp-specs]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#fault-proof-program

[book]: https://anton-rs.github.io/kona/

[op-labs]: https://github.com/ethereum-optimism
[bad-boi-labs]: https://github.com/BadBoiLabs

[g-output-root]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/glossary.md#l2-output-root
[g-derivation-pipeline]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/derivation.md#l2-chain-derivation-pipeline
[g-fault-proof-vm]: https://github.com/ethereum-optimism/optimism/blob/develop/specs/fault-proof.md#fault-proof-vm
