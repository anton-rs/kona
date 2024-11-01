# The `kona-derive` Derivation Pipeline

[`kona-derive`][kd] defines an entirely trait-abstracted, `no_std` derivation
pipeline for the OP Stack. It can be used through the [`Pipeline`][p] trait,
which is implemented for the concrete [`DerivationPipeline`][dp] object.

This document dives into the inner workings of the derivation pipeline, its
stages, and how to build and interface with Kona's pipeline. Other documents
in this section will provide a comprehensive overview of Derivation Pipeline
extensibility including trait-abstracted providers, custom stages, signaling,
and hardfork activation including multiplexed stages.

- [Swapping out a stage](./stages.md)
- [Defining a custom Provider](./providers.md)
- [Extending Pipeline Signals](./signals.md)
- [Implementing Hardfork Activations](./hardforks.md)


## What is a Derivation Pipeline?

Simply put, an OP Stack Derivation Pipeline transforms data on L1 into L2
payload attributes that can be executed to produce the canonical L2 block.

Within a pipeline, there are a set of stages that break up this transformation
further. When composed, these stages operate over the input data, sequentially
producing payload attributes.

In [`kona-derive`][kd], stages are architected using composition - each sequential
stage owns the previous one, forming a stack. For example, let's define stage A
as the first stage, accepting raw L1 input data, and stage C produces the pipeline
output - payload attributes. Stage B "owns" stage A, and stage C then owns stage B.
Using this example, the [`DerivationPipeline`][dp] type in [`kona-derive`][kd] only
holds stage C, since ownership of the other stages is nested within stage C.

> [!NOTE]
>
> In a future architecture of the derivation pipeline, stages could be made
> standalone such that communication between stages happens through channels.
> In a multi-threaded, non-fault-proof environment, these stages can then
> run in parallel since stage ownership is decoupled.


## Kona's Derivation Pipeline

The top-level stage in [`kona-derive`][kd] that produces
[`OpAttributesWithParent`][attributes] is the [`AttributesQueue`][attributes-queue].

Post-Holocene (the Holocene hardfork), the following stages are composed by
the [`DerivationPipeline`][dp].
- [`AttributesQueue`][attributes-queue]
   - [`BatchProvider`][batch-provider]
      - [`BatchStream`][batch-stream]
         - [`ChannelReader`][channel-reader]
            - [`ChannelProvider`][channel-provider]
               - [`FrameQueue`][frame-queue]
                  - [`L1Retrieval`][retrieval]
                     - [`L1Traversal`][traversal]

Notice, from top to bottom, each stage owns the stage nested below it.
Where the [`L1Traversal`][traversal] stage iterates over L1 data, the
[`AttributesQueue`][attributes-queue] stage produces
[`OpAttributesWithParent`][attributes], creating a function that transforms
L1 data into payload attributes.


## The [`Pipeline`][p] interface

Now that we've broken down the stages inside the [`DerivationPipeline`][dp]
type, let's move up another level to break down how the [`DerivationPipeline`][dp]
type functions itself. At the highest level, [`kona-derive`][kd] defines the
interface for working with the pipeline through the [`Pipeline`][p] trait.

[`Pipeline`][p] provides two core methods.
- `peek() -> Option<&OpAttributesWithParent>`
- `async step() -> StepResult`

Functionally, a pipeline can be "stepped" on, which attempts to derive
payload attributes from input data. Steps do not guarantee that payload attributes
are produced, they only attempt to advance the stages within the pipeline.

The `peek()` method provides a way to check if attributes are prepared.
Beyond `peek()` returning `Option::Some(&OpAttributesWithParent)`, the [`Pipeline`][p]
extends the [Iterator][iterator] trait, providing a way to consume the generated payload
attributes.


## Constructing a Derivation Pipeline

[`kona-derive`][kd] provides a [`PipelineBuilder`][builder] to abstract the complexity
of generics away from the downstream consumers. Below we provide an example for using
the [`PipelineBuilder`][builder] to instantiate a [`DerivationPipeline`][dp].

```rust
// Imports
use std::sync::Arc;
use op_alloy_protocol::BlockInfo;
use op_alloy_genesis::RollupConfig;
use kona_derive_alloy::prelude::*;

// Use a default rollup config.
let rollup_config = Arc::new(RollupConfig::default());

// Providers are instantiated to with localhost urls (`127.0.0.1`)
let chain_provider =
    AlloyChainProvider::new_http("http://127.0.0.1:8545".try_into().unwrap());
let l2_chain_provider = AlloyL2ChainProvider::new_http(
    "http://127.0.0.1:9545".try_into().unwrap(),
    rollup_config.clone(),
);
let beacon_client = OnlineBeaconClient::new_http("http://127.0.0.1:5555".into());
let blob_provider = OnlineBlobProvider::new(beacon_client, None, None);
let blob_provider = OnlineBlobProviderWithFallback::new(blob_provider, None);
let dap_source =
    EthereumDataSource::new(chain_provider.clone(), blob_provider, &rollup_config);
let builder = StatefulAttributesBuilder::new(
    rollup_config.clone(),
    l2_chain_provider.clone(),
    chain_provider.clone(),
);

// This is the starting L1 block for the pipeline.
//
// To get the starting L1 block for a given L2 block,
// use the `AlloyL2ChainProvider::l2_block_info_by_number`
// method to get the `L2BlockInfo.l1_origin`. This l1_origin
// is the origin that can be passed here.
let origin = BlockInfo::default();

// Build the pipeline using the `PipelineBuilder`.
// Alternatively, use the `new_online_pipeline` helper
// method provided by the `kona-derive-alloy` crate.
let pipeline = PipelineBuilder::new()
   .rollup_config(rollup_config.clone())
   .dap_source(dap_source)
   .l2_chain_provider(l2_chain_provider)
   .chain_provider(chain_provider)
   .builder(builder)
   .origin(origin)
   .build();

assert_eq!(pipeline.rollup_config, rollup_config);
assert_eq!(pipeline.origin(), Some(origin));
```


## Producing Payload Attributes

...

## Resets

...

## Learn More

[`kona-derive`][kd] is one implementation of the OP Stack derivation pipeline.

To learn more, it is highly encouraged to read the ["first" derivation pipeline][op-dp]
written in [golang][go]. It is often colloquially referred to as the "reference"
implementation and provides the basis for how much of Kona's derivation pipeline
was built.


## Provenance

> The lore do be bountiful.
>
> - Bard XVIII of the Gates of Logic

The kona project spawned out of the need to build a secondary fault proof for the OP Stack.
Initially, we sought to re-use [magi][magi]'s derivation pipeline, but the ethereum-rust
ecosystem moves quickly and [magi][magi] was behind by a generation of types - using
[ethers-rs] instead of new [alloy][alloy] types. Additionally, [magi][magi]'s derivation
pipeline was not `no_std` compatible - a hard requirement for running a rust fault proof
program on top of the RISCV or MIPS ISAs.

So, [@clabby][clabby] and [@refcell][refcell] stood up [kona][kona] in a few months.


<!-- Links -->

[builder]: https://docs.rs/kona-derive/latest/kona_derive/pipeline/struct.PipelineBuilder.html
[alloy]: https://github.com/alloy-rs/alloy
[ethers-rs]: https://github.com/gakonst/ethers-rs
[kona]: https://github.com/anton-rs/kona
[clabby]: https://github.com/clabby
[refcell]: https://github.com/refcell
[go]: https://go.dev/
[magi]: https://github.com/a16z/magi
[kd]: https://crates.io/crates/kona-derive
[iterator]: https://doc.rust-lang.org/nightly/core/iter/trait.Iterator.html
[p]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.Pipeline.html
[op-dp]: https://github.com/ethereum-optimism/optimism/tree/develop/op-node/rollup/derive
[dp]: https://docs.rs/kona-derive/latest/kona_derive/pipeline/struct.DerivationPipeline.html
[attributes]: https://docs.rs/op-alloy-rpc-types-engine/latest/op_alloy_rpc_types_engine/struct.OpAttributesWithParent.html

<!-- Pipeline Stages -->

[attributes-queue]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.AttributesQueue.html
[batch-provider]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.BatchProvider.html
[batch-stream]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.BatchStream.html
[channel-reader]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.ChannelReader.html
[channel-provider]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.ChannelProvider.html
[frame-queue]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.FrameQueue.html
[retrieval]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.L1Retrieval.html
[traversal]: https://docs.rs/kona-derive/latest/kona_derive/stages/struct.L1Traversal.html
