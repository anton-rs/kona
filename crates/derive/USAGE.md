## Usage

```rust,ignore
use std::sync::Arc;
use kona_derive::online::*;
use kona_derive::pipeline::*;
use kona_primitives::{BlockInfo, L2BlockInfo, RollupConfig};

// TODO(refcell): replace this will a rollup config
// fetched from the superchain-registry via network id.
let rollup_config = Arc::new(RollupConfig::default());

// Creates a new chain provider using the `L1_RPC_URL` environment variable.
let l1_rpc_url = std::env::var("L1_RPC_URL").expect("L1_RPC_URL must be set");
let chain_provider = AlloyChainProvider::new_http(l1_rpc_url.parse().unwrap());

// Creates a new l2 chain provider using the `L2_RPC_URL` environment variable.
let l2_rpc_url = std::env::var("L2_RPC_URL").expect("L2_RPC_URL must be set");
let l2_chain_provider = AlloyL2ChainProvider::new_http(l2_rpc_url.parse().unwrap(), rollup_config.clone());

// Create the beacon client used to fetch blob data.
let beacon_url = std::env::var("BEACON_URL").expect("BEACON_URL must be set");
let beacon_client = OnlineBeaconClient::new_http(beacon_url.parse().unwrap());

// Build the online blob provider.
let blob_provider = OnlineBlobProvider::<_, SimpleSlotDerivation>::new(true, beacon_client, None, None);

// Build the ethereum data source
let dap_source = EthereumDataSource::new(chain_provider.clone(), blob_provider, &rollup_config);

// The payload attributes builder that is stateful. 
let attributes_builder = StatefulAttributesBuilder::new(rollup_config.clone(), l2_chain_provider.clone(), chain_provider.clone());

// Build the pipeline.
let pipeline = PipelineBuilder::new()
    .rollup_config(rollup_config)
    .dap_source(dap_source)
    .l2_chain_provider(l2_chain_provider)
    .chain_provider(chain_provider)
    .builder(attributes_builder)
    .build();

// The pipeline should be at the default state.
assert_eq!(pipeline.tip, BlockInfo::default());
assert_eq!(pipeline.cursor, L2BlockInfo::default());
```
