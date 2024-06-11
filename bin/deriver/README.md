
# kona-deriver

A simple program that executes the [derivation pipeline][derive] over L1 Blocks.

[derive]: https://github.com/ethereum-optimism/kona/tree/main/crates/derive

## Usage

Required environment variables.

`VERBOSITY_LEVEL`: The level of verbosity for the pipeline.
`L2_RPC_URL`: The RPC URL used to validate the derived payload attributes and span batches.
`L1_RPC_URL`: Used by the L1 Traversal Stage to grab new L1 Blocks. This can point to the local reth L1 node http endpoint. The online `AlloyChainProvider` that queries these blocks over RPC can be changed for some new provider implementation that just pulls the blocks from disk or the committed chain. Note, this new provider must implement the `ChainProvider` trait that the L1 Traversal Stage uses to pull in the L1 Blocks.
`BEACON_URL`: The beacon provider that is used to fetch blobs. This could probably also be optimized to pull in blobs when an L1 block is committed by grabbing the blob sidecars from the `Chain` passed into the Execution Extension's commit function.

Run the example with these environment variables set. (Example shown below).

```
VERBOSITY_LEVEL=4 cargo run
```
