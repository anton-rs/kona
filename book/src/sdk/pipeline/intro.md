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

```rust,ignore
// Imports
use std::sync::Arc;
use maili_protocol::BlockInfo;
use op_alloy_genesis::RollupConfig;
use hilo_providers_alloy::*;

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

Since the [`Pipeline`][p] trait extends the [`Iterator`][iterator] trait,
producing [`OpAttributesWithParent`][attributes] is as simple as as calling
[`Iterator::next()`][next] method on the [`DerivationPipeline`][dp].

Extending the example from above, producing the attributes is shown below.

```rust
// Import the iterator trait to show where `.next` is sourced.
use core::iter::Iterator;

// ...
// example from above constructing the pipeline
// ...

let attributes = pipeline.next();

// Since we haven't stepped on the pipeline,
// there shouldn't be any payload attributes prepared.
assert!(attributes.is_none());
```

As demonstrated, the pipeline won't have any payload attributes
without having been "stepped" on. Naively, we can continuously
step on the pipeline until attributes are ready, and then consume them.

```rust
// Import the iterator trait to show where `.next` is sourced.
use core::iter::Iterator;

// ...
// example from constructing the pipeline
// ...

// Continuously step on the pipeline until attributes are prepared.
let l2_safe_head = L2BlockInfo::default();
loop {
   if matches!(pipeline.step(l2_safe_head).await, StepResult::PreparedAttributes) {
      // The pipeline has succesfully prepared payload attributes, break the loop.
      break;
   }
}

// Since the loop is only broken once attributes are prepared,
// this must be `Option::Some`.
let attributes = pipeline.next().expect("Must contain payload attributes");

// The parent of the prepared payload attributes should be
// the l2 safe head that we "stepped on".
assert_eq!(attributes.parent, l2_safe_head);
```

Importantly, the above is not sufficient logic to produce payload attributes and drive
the derivation pipeline. There are multiple different `StepResult`s to handle when
stepping on the pipeline, including advancing the origin, re-orgs, and pipeline resets.
In the next section, pipeline resets are outlined.

For an up-to-date driver that runs the derivation pipeline as part of the fault proof
program, reference kona's [client driver][driver].


## Resets

When stepping on the [`DerivationPipeline`][dp] produces a reset error, the driver
of the pipeline must perform a reset on the pipeline. This is done by sending a "signal"
through the [`DerivationPipeline`][dp]. Below demonstrates this.

```rust
// Import the iterator trait to show where `.next` is sourced.
use core::iter::Iterator;

// ...
// example from constructing the pipeline
// ...

// Continuously step on the pipeline until attributes are prepared.
let l2_safe_head = L2BlockInfo::default();
loop {
   match pipeline.step(l2_safe_head).await {
      StepResult::StepFailed(e) | StepResult::OriginAdvanceErr(e) => {
         match e {
            PipelineErrorKind::Reset(e) => {
               // Get the system config from the provider.
               let system_config = l2_chain_provider
                  .system_config_by_number(
                     l2_safe_head.block_info.number,
                     rollup_config.clone(),
                  )
                  .await?;
               // Reset the pipeline to the initial L2 safe head and L1 origin.
               self.pipeline
                  .signal(
                      ResetSignal {
                          l2_safe_head: l2_safe_head,
                          l1_origin: pipeline
                              .origin()
                              .ok_or_else(|| anyhow!("Missing L1 origin"))?,
                          system_config: Some(system_config),
                      }
                      .signal(),
                  )
                  .await?;
               // ...
            }
            _ => { /* Handling left to the driver */ }
         }
      }
      _ => { /* Handling left to the driver */ }
   }
}
```


## Learn More

[`kona-derive`][kd] is one implementation of the OP Stack derivation pipeline.

To learn more, it is highly encouraged to read the ["first" derivation pipeline][op-dp]
written in [golang][go]. It is often colloquially referred to as the "reference"
implementation and provides the basis for how much of Kona's derivation pipeline
was built.


## Provenance

> The lore do be bountiful.
>
> - Bard XVIII of the Logic Gates

The kona project spawned out of the need to build a secondary fault proof for the OP Stack.
Initially, we sought to re-use [magi][magi]'s derivation pipeline, but the ethereum-rust
ecosystem moves quickly and [magi][magi] was behind by a generation of types - using
[ethers-rs] instead of new [alloy][alloy] types. Additionally, [magi][magi]'s derivation
pipeline was not `no_std` compatible - a hard requirement for running a rust fault proof
program on top of the RISCV or MIPS ISAs.

So, [@clabby][clabby] and [@refcell][refcell] stood up [kona][kona] in a few months.


<!-- Links -->

[driver]: https://github.com/op-rs/kona/blob/main/bin/client/src/l1/driver.rs#L74
[next]: https://doc.rust-lang.org/nightly/core/iter/trait.Iterator.html#tymethod.next
[builder]: https://docs.rs/kona-derive/latest/kona_derive/pipeline/struct.PipelineBuilder.html
[alloy]: https://github.com/alloy-rs/alloy
[ethers-rs]: https://github.com/gakonst/ethers-rs
[kona]: https://github.com/op-rs/kona
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
