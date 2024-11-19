# Custom Backends

## Understanding the OP Stack STF

The OP Stack state transition is comprised of two primary components:

- **The [derivation pipeline](https://specs.optimism.io/protocol/derivation.html)** (`kona-derive`)
  - Responsible for deriving L2 chain state from the DA layer.
- **The [execution engine](https://specs.optimism.io/protocol/exec-engine.html#l2-execution-engine)** (`kona-executor`)
  - Responsible for the execution of transactions and state commitments.
  - Ensures correct application of derived L2 state.

To prove the correctness of the state transition, Kona composes these two components:

- It combines the derivation of the L2 chain with its execution in the same process.
- It pulls in necessary data from sources to complete the STF, verifiably unrolling the input commitments along the way.

`kona-client` serves as an implementation of this process, capable of deriving and executing a single L2 block in a
verifiable manner.

> 📖 Why just a single block by default?
>
> On the OP Stack, we employ an interactive bisection game that narrows in on the disagreed upon block -> block state
> transition before requiring a fault proof to be ran. Because of this, the default implementation only serves
> to derive and execute the single block that the participants of the bisection game landed on.

## Backend Traits

Covered in the [FPVM Backend](./fpvm-backend.md) section of the book, `kona-client` ships with an implementation of
`kona-derive` and `kona-executor`'s data source traits which pull in data over the [PreimageOracle ABI][preimage-specs].

However, running `kona-client` on top of a different verifiable environment, i.e. a zkVM or TEE, is also possible
through custom implementations of these data source traits.

[`op-succinct`](https://github.com/succinctlabs/op-succinct) is an excellent example of both a custom backend and a custom
program, implementing both `kona-derive` and `kona-executor`'s data source traits backed by [sp1_lib::io](https://docs.rs/sp1-lib/latest/sp1_lib/io/index.html)
in order to:

1. Execute `kona-client` verbatim, proving a single block's derivation and execution on SP-1.
1. Derive and execute an entire [Span Batch](https://specs.optimism.io/protocol/delta/span-batches.html#span-batches)
   worth of L2 blocks, using `kona-derive` and `kona-executor`.

This section of the book outlines how you can do the same for a different platform.

### Custom `kona-derive` sources

Before getting started, we need to create custom implementations of the following traits:

| Trait                                                                                                 | Description                                                                                                                         |
| ----------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| [`ChainProvider`](https://docs.rs/kona-derive/latest/kona_derive/traits/trait.ChainProvider.html)     | The `ChainProvider` trait describes the minimal interface for fetching data from L1 during L2 chain derivation.                     |
| [`L2ChainProvider`](https://docs.rs/kona-derive/latest/kona_derive/traits/trait.L2ChainProvider.html) | The `ChainProvider` trait describes the minimal interface for fetching data from the safe L2 chain during L2 chain derivation.      |
| [`BlobProvider`](https://docs.rs/kona-derive/latest/kona_derive/traits/trait.BlobProvider.html)       | The `BlobProvider` trait describes an interface for fetching EIP-4844 blobs from the L1 consensus layer during L2 chain derivation. |

Once these are implemented, constructing the pipeline is as simple as passing in the data sources to the `PipelineBuilder`. Keep in mind the requirements for validation of incoming data, depending on your platform. For example, programs
targeting zkVMs must constrain that the incoming data is indeed valid, whereas fault proof programs can offload this validation to the on-chain implementation of the host.

```rs
let chain_provider = ...;
let l2_chain_provider = ...;
let blob_provider = ...;
let l1_origin = ...;

let cfg = Arc::new(RollupConfig::default());
let attributes = StatefulAttributesBuilder::new(
   cfg.clone(),
   l2_chain_provider.clone(),
   chain_provider.clone(),
);
let dap = EthereumDataSource::new(
   chain_provider.clone(),
   blob_provider,
   cfg.as_ref()
);

// Construct a new derivation pipeline.
let pipeline = PipelineBuilder::new()
   .rollup_config(cfg)
   .dap_source(dap)
   .l2_chain_provider(l2_chain_provider)
   .chain_provider(chain_provider)
   .builder(attributes)
   .origin(l1_origin)
   .build();
```

From here, a custom derivation driver is needed to produce the desired execution payload(s). An example of this for
`kona-client` can be found in the [DerivationDriver](https://github.com/anton-rs/kona/blob/main/bin/client/src/l1/driver.rs#L77).

### `kona-mpt` / `kona-executor` sources

Before getting started, we need to create custom implementations of the following traits:

| Trait                                                                                | Description                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`TrieDBFetcher`](https://docs.rs/kona-mpt/latest/kona_mpt/trait.TrieDBFetcher.html) | The `TrieDBFetcher` trait describes the interface for fetching trie node preimages and chain information while executing a payload on the L2 chain.                                                                                                                                                                                |
| [`TrieDBHinter`](https://docs.rs/kona-mpt/latest/kona_mpt/trait.TrieDBHinter.html)   | The `TrieDBHinter` trait describes the interface for requesting the host program to prepare trie proof preimages for the client's consumption. For targets with upfront witness generation, i.e. zkVMs, a no-op hinter is exported as [`NoopTrieDBHinter`](https://docs.rs/kona-mpt/latest/kona_mpt/struct.NoopTrieDBHinter.html). |

Once we have those, the `StatelessL2BlockExecutor` can be constructed like so:

```rust
let cfg = RollupConfig::default();
let provider = ...;
let hinter = ...;

let executor = StatelessL2BlockExecutor::builder(&cfg, provider, hinter)
   .with_parent_header(...)
   .build();

let header = executor.execute_payload(...).expect("Failed execution");
```

### Bringing it Together

Once your custom backend traits for both `kona-derive` and `kona-executor` have been implemented,
your final binary may look something like [that of `kona-client`'s](https://github.com/anton-rs/kona/blob/main/bin/client/src/kona.rs).
Alternatively, if you're looking to prove a wider range of blocks, [`op-succinct`'s `range` program](https://github.com/succinctlabs/op-succinct/tree/main/programs/range)
offers a good example of running the pipeline and executor across a string of contiguous blocks.

{{ #include ../links.md }}
