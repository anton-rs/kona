//! Stateless L2 payload executor that captures the witness it gathers in a [KeyValueStore].

use alloy_consensus::{Header, Sealed};
use alloy_primitives::{Bytes, Sealable, B256};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rlp::Decodable;
use kona_executor::{ExecutorResult, StatelessL2BlockExecutor, TrieDBProvider};
use kona_host::KeyValueStore;
use kona_mpt::{NoopTrieHinter, TrieNode, TrieProvider};
use kona_preimage::{PreimageKey, PreimageKeyType};
use maili_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;
use std::sync::Arc;
use tokio::{runtime::Handle, sync::RwLock};

/// A stateless L2 payload executor that gathers an execution witness in a [KeyValueStore].
pub struct WitnessCollector<'a, K: KeyValueStore + ?Sized> {
    /// The rollup configuration.
    config: &'a RollupConfig,
    /// The parent header.
    parent_header: Sealed<Header>,
    /// The EVM provider
    evm_provider: StatelessEVMProvider<K>,
}

impl<'a, K: KeyValueStore + ?Sized> WitnessCollector<'a, K> {
    /// Creates a new [WitnessCollector] with the given configuration.
    pub fn new(
        config: &'a RollupConfig,
        parent_header: Sealed<Header>,
        l1_provider: ReqwestProvider,
        l2_provider: ReqwestProvider,
        witness_store: Arc<RwLock<K>>,
    ) -> Self {
        let evm_provider = StatelessEVMProvider { l1_provider, l2_provider, witness_store };
        Self { config, parent_header, evm_provider }
    }

    /// Executes the payload, gathering the witness in the [KeyValueStore], and returns the sealed [Header].
    pub fn execute_payload(self, attrs: OpPayloadAttributes) -> ExecutorResult<Sealed<Header>> {
        let mut executor =
            StatelessL2BlockExecutor::builder(&self.config, self.evm_provider, NoopTrieHinter)
                .with_parent_header(self.parent_header)
                .build();
        executor.execute_payload(attrs).cloned().map(|h| h.seal_slow())
    }
}

/// A provider that fetches trie nodes and EVM context from remote providers and stores results
/// in a [KeyValueStore].
struct StatelessEVMProvider<K: ?Sized> {
    /// The L1 provider.
    l1_provider: ReqwestProvider,
    /// The L2 provider.
    l2_provider: ReqwestProvider,
    /// The witness store.
    witness_store: Arc<RwLock<K>>,
}

impl<K> TrieProvider for StatelessEVMProvider<K>
where
    K: KeyValueStore + ?Sized,
{
    type Error = anyhow::Error;

    fn trie_node_by_hash(&self, key: B256) -> Result<TrieNode, Self::Error> {
        // Fetch the preimage from the L2 chain provider.
        let preimage: Bytes = tokio::task::block_in_place(move || {
            Handle::current().block_on(async {
                let preimage: Bytes =
                    self.l1_provider.client().request("debug_dbGet", &[key]).await?;

                self.witness_store.write().await.set(
                    PreimageKey::new(*key, PreimageKeyType::Keccak256).into(),
                    preimage.clone().into(),
                )?;
                Ok::<_, anyhow::Error>(preimage)
            })
        })?;

        // Decode the preimage into a trie node.
        TrieNode::decode(&mut preimage.as_ref()).map_err(Into::into)
    }
}

impl<K> TrieDBProvider for StatelessEVMProvider<K>
where
    K: KeyValueStore + ?Sized,
{
    fn bytecode_by_hash(&self, hash: B256) -> Result<Bytes, Self::Error> {
        // geth hashdb scheme code hash key prefix
        const CODE_PREFIX: u8 = b'c';

        // Fetch the preimage from the L2 chain provider.
        let preimage: Bytes = tokio::task::block_in_place(move || {
            Handle::current().block_on(async {
                // Attempt to fetch the code from the L2 chain provider.
                let code_hash = [&[CODE_PREFIX], hash.as_slice()].concat();
                let code = self
                    .l2_provider
                    .client()
                    .request::<&[Bytes; 1], Bytes>("debug_dbGet", &[code_hash.into()])
                    .await;

                // Check if the first attempt to fetch the code failed. If it did, try fetching the
                // code hash preimage without the geth hashdb scheme prefix.
                let code = match code {
                    Ok(code) => code,
                    Err(_) => {
                        self.l2_provider
                            .client()
                            .request::<&[B256; 1], Bytes>("debug_dbGet", &[hash])
                            .await?
                    }
                };

                self.witness_store.write().await.set(
                    PreimageKey::new(*hash, PreimageKeyType::Keccak256).into(),
                    code.clone().into(),
                )?;

                Ok::<_, anyhow::Error>(code)
            })
        })?;

        Ok(preimage)
    }

    fn header_by_hash(&self, hash: B256) -> Result<Header, Self::Error> {
        let encoded_header: Bytes = tokio::task::block_in_place(move || {
            Handle::current().block_on(async {
                let preimage: Bytes =
                    self.l2_provider.client().request("debug_getRawHeader", &[hash]).await?;

                self.witness_store.write().await.set(
                    PreimageKey::new(*hash, PreimageKeyType::Keccak256).into(),
                    preimage.clone().into(),
                )?;

                Ok::<_, anyhow::Error>(preimage)
            })
        })?;

        // Decode the Header.
        Header::decode(&mut encoded_header.as_ref()).map_err(Into::into)
    }
}
