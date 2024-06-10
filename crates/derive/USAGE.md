## Usage

```rust
use alloc::sync::Arc;
use alloy_provider::ReqwestProvider;
use alloy_rpc_client::RpcClient;
use alloy_transport_http::Http;
use kona_derive::{
     online::{
         AlloyChainProvider, AlloyL2ChainProvider, OnlineBeaconClient, OnlineBlobProvider,
         SimpleSlotDerivation,
     },
     pipeline::*,
 };
use kona_primitives::L2BlockInfo;
use reqwest::Client;

 // Creates a new chain provider using the `L1_RPC_URL` environment variable.
 let l1_rpc_url = std::env::var(\"L1_RPC_URL\").expect(\"L1_RPC_URL must be set\");
 let l1_rpc_url = l1_rpc_url.parse().unwrap();
 let http = Http::<Client>::new(l1_rpc_url);
 let chain_provider = AlloyChainProvider::new(ReqwestProvider::new(RpcClient::new(http, true)));

 // Creates a new l2 chain provider using the `L2_RPC_URL` environment variable.
 let l2_rpc_url = std::env::var(\"L2_RPC_URL\").expect(\"L2_RPC_URL must be set\");
 let l2_rpc_url = l2_rpc_url.parse().unwrap();
 let http = Http::<Client>::new(l2_rpc_url);
 let l2_chain_provider =
     AlloyL2ChainProvider::new(ReqwestProvider::new(RpcClient::new(http, true)));

 // TODO(refcell): replace this will a rollup config
 // fetched from the superchain-registry via network id.
 let rollup_config = Arc::new(RollupConfig::default());

 // Create the beacon client used to fetch blob data.
 let beacon_url = std::env::var(\"BEACON_URL\").expect(\"BEACON_URL must be set\");
 let beacon_url = beacon_url.parse().unwrap();
 let http = Http::<Client>::new(beacon_url);
 let beacon_client = OnlineBeaconClient::new(ReqwestProvider::new(RpcClient::new(http, true)));

 // Build the online blob provider.
 let blob_provider: OnlineBlobProvider<_, SimpleSlotDerivation> =
     OnlineBlobProvider::new(true, beacon_client, None, None);

 // Build the ethereum data source
 let dap_source = EthereumDataSource::new(chain_provider.clone(), blob_provider, &rollup_config);

 let builder = PipelineBuilder::new();
 let pipeline = builder
     .rollup_config(rollup_config)
     .dap_source(dap_source)
     .l2_chain_provider(l2_chain_provider)
     .chain_provider(chain_provider)
     .builder(OnlineAttributesBuilder::new())
     .reset(ResetProvider::new())
     .start_cursor(L2BlockInfo::default())
     .build();

 assert_eq!(pipeline.needs_reset, false);
```
