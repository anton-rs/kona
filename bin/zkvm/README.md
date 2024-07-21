# SP1 Project Template

This is a template for creating an end-to-end [SP1](https://github.com/succinctlabs/sp1) project
that can generate a proof of any RISC-V program.

## Requirements

- [Rust](https://rustup.rs/)
- [SP1](https://succinctlabs.github.io/sp1/getting-started/install.html)

## Standard Proof Generation

> [!WARNING]
> You will need at least 16GB RAM to generate the default proof.

Generate the proof for your program using the standard prover.

```sh
cd script
RUST_LOG=info cargo run --bin prove --release
```

## EVM-Compatible Proof Generation & Verification

> [!WARNING]
> You will need at least 128GB RAM to generate the PLONK proof.

Generate the proof that is small enough to be verified on-chain and verifiable by the EVM. This command also generates a fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.

```sh
cd script
RUST_LOG=info cargo run --bin prove --release -- --evm
```

## Using the Prover Network

Make a copy of the example environment file:

```sh
cp .env.example .env
```

Then, set the `SP1_PROVER` environment variable to `network` and set the `SP1_PRIVATE_KEY` environment variable to your whitelisted private key. For more information, see the [setup guide](https://docs.succinct.xyz/prover-network/setup.html).
