# trusted-sync

An example that executes the [derivation pipeline][derive] over L1 Blocks and validates payloads using a trusted rpc endpoint.

[derive]: https://github.com/ethereum-optimism/kona/tree/main/crates/derive

## Usage

From the `kona` root directory, specify the example with `cargo run -p trusted-sync`.

Example below (uses the environment variables for the rpc cli flags since they are not specified).

```
cargo run -p trusted-sync -vvv
```

Optional flags (defaults to environment variables).

`-v`: Verbosity (`-v`, `-vv`, `-vvv`, `-vvvv`).
`--start-l2-block`: An L2 Block Number to use as the starting point for derivation.
`--l2-rpc-url` (`L2_RPC_URL`): The RPC URL used to validate the derived payload attributes and span batches.
`--l1-rpc-url` (`L1_RPC_URL`): Used by the L1 Traversal Stage to grab new L1 Blocks. This can point to the local reth L1 node http endpoint. The online `AlloyChainProvider` that queries these blocks over RPC can be changed for some new provider implementation that just pulls the blocks from disk or the committed chain. Note, this new provider must implement the `ChainProvider` trait that the L1 Traversal Stage uses to pull in the L1 Blocks.
`--beacon-url` (`BEACON_URL`): The beacon provider that is used to fetch blobs. This could probably also be optimized to pull in blobs when an L1 block is committed by grabbing the blob sidecars from the `Chain` passed into the Execution Extension's commit function.
