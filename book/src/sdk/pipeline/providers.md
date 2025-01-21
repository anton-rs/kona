# Trait-abstracted Providers

Kona's derivation pipeline pulls in data from sources that are trait
abstracted so the pipeline can be generic over various data sources.
Note, "data sources" is used interchangeably with "trait-abstracted
providers" for the purpose of this document.

The key traits required for the pipeline are the following.

- [`ChainProvider`][chain-provider]
- [`L2ChainProvider`][l2-chain-provider]
- [`DataAvailabilityProvider`][dap]

The [`kona-derive-alloy`][kda] crate provides `std` implementations
of these traits using [Alloy][alloy]'s `reqwest`-backed providers.

## Provider Usage

Although trait-abstracted Providers are used throughout the pipeline and
its stages, the [`PipelineBuilder`][builder] makes constructing the pipeline
generic over the providers. An example is shown below, where the three
required trait implementations are the providers stubbed with `todo!()`.

```rust
use std::sync::Arc;
use maili_genesis::RollupConfig;
use kona_derive::pipeline::PipelineBuilder;
use kona_derive::attributes::StatefulAttributesBuilder;

// The rollup config for your chain.
let cfg = Arc::new(RollupConfig::default());

// Must implement the `ChainProvider` trait.
let chain_provider = todo!("your chain provider");

// Must implement the `L2ChainProvider` trait.
let l2_chain_provider = todo!("your l2 chain provider");

// Must implement the `DataAvailabilityProvider` trait.
let dap = todo!("your data availability provider");

// Generic over the providers.
let attributes = StatefulAttributesBuilder::new(
   cfg.clone(),
   l2_chain_provider.clone(),
   chain_provider.clone(),
);

// Construct a new derivation pipeline.
let pipeline = PipelineBuilder::new()
   .rollup_config(cfg)
   .dap_source(dap)
   .l2_chain_provider(l2_chain_provider)
   .chain_provider(chain_provider)
   .builder(attributes)
   .origin(BlockInfo::default())
   .build();
```

## Implementing a Custom Data Availability Provider

> Notice
>
> The only required method for the [`DataAvailabilityProvider`][dap]
> trait is the [`next`][next] method.

```rust
use async_trait::async_trait;
use alloy_primitives::Bytes;
use maili_protocol::BlockInfo;
use kona_derive::traits::DataAvailabilityProvider;
use kona_derive::errors::PipelineResult;

/// ExampleAvail
///
/// An example implementation of the `DataAvailabilityProvider` trait.
#[derive(Debug)]
pub struct ExampleAvail {
   // Place your data in here
}

#[async_trait]
impl DataAvailabilityProvider for ExampleAvail {
   type Item = Bytes;

   async fn next(&self, block_ref: &BlockInfo) -> PipelineResult<Self::Item> {
      todo!("return an AsyncIterator implementation here")
   }
}
```


<!-- Links -->

[dap]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.DataAvailabilityProvider.html
[next]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.DataAvailabilityProvider.html#tymethod.next
[builder]: https://docs.rs/kona-derive/latest/kona_derive/pipeline/struct.PipelineBuilder.html
[alloy]: https://github.com/alloy-rs/alloy
[kda]: https://crates.io/crates/kona-derive-alloy
[chain-provider]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.ChainProvider.html
[l2-chain-provider]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.L2ChainProvider.html
[dap]: https://docs.rs/kona-derive/latest/kona_derive/traits/trait.DataAvailabilityProvider.html
