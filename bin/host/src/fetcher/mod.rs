//! This module contains the [Fetcher] struct, which is responsible for fetching preimages from a
//! remote source.

use crate::{kv::KeyValueStore, util};
use alloy_consensus::TxEnvelope;
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{keccak256, Address, Bytes, B256};
use alloy_provider::{Provider, ReqwestProvider};
use alloy_rpc_types::{Block, BlockTransactions};
use anyhow::{anyhow, Result};
use kona_preimage::{PreimageKey, PreimageKeyType};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

mod hint;
pub use hint::HintType;

mod precompiles;

/// The [Fetcher] struct is responsible for fetching preimages from a remote source.
pub struct Fetcher<KV>
where
    KV: KeyValueStore + ?Sized,
{
    /// Key-value store for preimages.
    kv_store: Arc<RwLock<KV>>,
    /// L1 chain provider.
    l1_provider: ReqwestProvider,
    /// L2 chain provider.
    /// TODO: OP provider, N = Optimism
    #[allow(unused)]
    l2_provider: ReqwestProvider,
    /// The last hint that was received. [None] if no hint has been received yet.
    last_hint: Option<String>,
}

impl<KV> Fetcher<KV>
where
    KV: KeyValueStore + ?Sized,
{
    /// Create a new [Fetcher] with the given [KeyValueStore].
    pub fn new(
        kv_store: Arc<RwLock<KV>>,
        l1_provider: ReqwestProvider,
        l2_provider: ReqwestProvider,
    ) -> Self {
        Self { kv_store, l1_provider, l2_provider, last_hint: None }
    }

    /// Set the last hint to be received.
    pub fn hint(&mut self, hint: &str) {
        debug!(target: "fetcher", "Received hint: {hint}");
        self.last_hint = Some(hint.to_string());
    }

    /// Get the preimage for the given key.
    pub async fn get_preimage(&self, key: B256) -> Result<Vec<u8>> {
        debug!(target: "fetcher", "Pre-image requested. Key: {key}");

        // Acquire a read lock on the key-value store.
        let kv_lock = self.kv_store.read().await;
        let mut preimage = kv_lock.get(key);

        // Drop the read lock before beginning the retry loop.
        drop(kv_lock);

        // Use a loop to keep retrying the prefetch as long as the key is not found
        while preimage.is_none() && self.last_hint.is_some() {
            let hint = self.last_hint.as_ref().expect("Cannot be None");
            self.prefetch(hint).await?;

            let kv_lock = self.kv_store.read().await;
            preimage = kv_lock.get(key);
        }

        preimage.ok_or_else(|| anyhow!("Preimage not found."))
    }

    /// Fetch the preimage for the given hint and insert it into the key-value store.
    async fn prefetch(&self, hint: &str) -> Result<()> {
        let (hint_type, hint_data) = util::parse_hint(hint)?;
        debug!(target: "fetcher", "Fetching hint: {hint_type} {hint_data}");

        match hint_type {
            HintType::L1BlockHeader => {
                // Validate the hint data length.
                if hint_data.len() != 32 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                // Fetch the raw header from the L1 chain provider.
                let hash: B256 = hint_data
                    .as_ref()
                    .try_into()
                    .map_err(|e| anyhow!("Failed to convert bytes to B256: {e}"))?;
                let raw_header: Bytes = self
                    .l1_provider
                    .client()
                    .request("debug_getRawHeader", [hash])
                    .await
                    .map_err(|e| anyhow!(e))?;

                // Acquire a lock on the key-value store and set the preimage.
                let mut kv_lock = self.kv_store.write().await;
                kv_lock.set(
                    PreimageKey::new(*hash, PreimageKeyType::Keccak256).into(),
                    raw_header.into(),
                );
            }
            HintType::L1Transactions => {
                // Validate the hint data length.
                if hint_data.len() != 32 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                // Fetch the block from the L1 chain provider and store the transactions within its
                // body in the key-value store.
                let hash: B256 = hint_data
                    .as_ref()
                    .try_into()
                    .map_err(|e| anyhow!("Failed to convert bytes to B256: {e}"))?;
                let Block { transactions, .. } = self
                    .l1_provider
                    .get_block_by_hash(hash, true)
                    .await
                    .map_err(|e| anyhow!("Failed to fetch block: {e}"))?
                    .ok_or(anyhow!("Block not found."))?;
                self.store_transactions(transactions).await?;
            }
            HintType::L1Receipts => {
                // Validate the hint data length.
                if hint_data.len() != 32 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                // Fetch the receipts from the L1 chain provider and store the receipts within the
                // key-value store.
                let hash: B256 = hint_data
                    .as_ref()
                    .try_into()
                    .map_err(|e| anyhow!("Failed to convert bytes to B256: {e}"))?;
                let raw_receipts: Vec<Bytes> = self
                    .l1_provider
                    .client()
                    .request("debug_getRawReceipts", [hash])
                    .await
                    .map_err(|e| anyhow!(e))?;
                self.store_trie_nodes(raw_receipts.as_slice()).await?;
            }
            HintType::L1Blob => todo!(),
            HintType::L1Precompile => {
                // Validate the hint data length.
                if hint_data.len() < 20 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                // Fetch the precompile address from the hint data.
                let precompile_address = Address::from_slice(&hint_data.as_ref()[..20]);
                let precompile_input = hint_data[20..].to_vec();
                let input_hash = keccak256(hint_data.as_ref());

                let result = match precompiles::execute(precompile_address, precompile_input) {
                    Ok(raw_res) => {
                        let mut res = Vec::with_capacity(1 + raw_res.len());
                        res.push(0x01); // success type byte
                        res.extend_from_slice(&raw_res);
                        res
                    }
                    Err(_) => {
                        // failure type byte
                        vec![0u8; 1]
                    }
                };

                // Acquire a lock on the key-value store and set the preimages.
                let mut kv_lock = self.kv_store.write().await;
                kv_lock.set(
                    PreimageKey::new(*input_hash, PreimageKeyType::Keccak256).into(),
                    hint_data.into(),
                );
                kv_lock
                    .set(PreimageKey::new(*input_hash, PreimageKeyType::Precompile).into(), result);
            }
            HintType::L2BlockHeader => todo!(),
            HintType::L2Transactions => todo!(),
            HintType::L2Code => todo!(),
            HintType::L2Output => todo!(),
            HintType::L2StateNode => todo!(),
            HintType::L2AccountProof => {
                if hint_data.len() != 8 + 20 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                let block_number = u64::from_be_bytes(
                    hint_data.as_ref()[..8]
                        .try_into()
                        .map_err(|e| anyhow!("Error converting hint data to u64: {e}"))?,
                );
                let address = Address::from_slice(&hint_data.as_ref()[8..]);

                let proof_response = self
                    .l2_provider
                    .get_proof(address, Default::default(), block_number.into())
                    .await
                    .map_err(|e| anyhow!("Failed to fetch account proof: {e}"))?;

                let mut kv_write_lock = self.kv_store.write().await;

                // Write the account proof nodes to the key-value store.
                proof_response.account_proof.into_iter().for_each(|node| {
                    let node_hash = keccak256(node.as_ref());
                    let key = PreimageKey::new(*node_hash, PreimageKeyType::Keccak256);
                    kv_write_lock.set(key.into(), node.into());
                });
            }
            HintType::L2AccountStorageProof => {
                if hint_data.len() != 8 + 20 + 32 {
                    anyhow::bail!("Invalid hint data length: {}", hint_data.len());
                }

                let block_number = u64::from_be_bytes(
                    hint_data.as_ref()[..8]
                        .try_into()
                        .map_err(|e| anyhow!("Error converting hint data to u64: {e}"))?,
                );
                let address = Address::from_slice(&hint_data.as_ref()[8..]);
                let slot = B256::from_slice(&hint_data.as_ref()[28..]);

                let mut proof_response = self
                    .l2_provider
                    .get_proof(address, vec![slot], block_number.into())
                    .await
                    .map_err(|e| anyhow!("Failed to fetch account proof: {e}"))?;

                let mut kv_write_lock = self.kv_store.write().await;

                // Write the account proof nodes to the key-value store.
                proof_response.account_proof.into_iter().for_each(|node| {
                    let node_hash = keccak256(node.as_ref());
                    let key = PreimageKey::new(*node_hash, PreimageKeyType::Keccak256);
                    kv_write_lock.set(key.into(), node.into());
                });

                // Write the storage proof nodes to the key-value store.
                let storage_proof = proof_response.storage_proof.remove(0);
                storage_proof.proof.into_iter().for_each(|node| {
                    let node_hash = keccak256(node.as_ref());
                    let key = PreimageKey::new(*node_hash, PreimageKeyType::Keccak256);
                    kv_write_lock.set(key.into(), node.into());
                });
            }
        }

        Ok(())
    }

    /// Stores a list of [BlockTransactions] in the key-value store.
    async fn store_transactions(&self, transactions: BlockTransactions) -> Result<()> {
        match transactions {
            BlockTransactions::Full(transactions) => {
                let encoded_transactions = transactions
                    .into_iter()
                    .map(|tx| {
                        let envelope: TxEnvelope = tx.try_into().map_err(|e| {
                            anyhow!(
                                "Failed to convert RPC transaction into consensus envelope: {e}"
                            )
                        })?;

                        Ok::<_, anyhow::Error>(envelope.encoded_2718())
                    })
                    .collect::<Result<Vec<_>>>()?;

                self.store_trie_nodes(encoded_transactions.as_slice()).await
            }
            _ => anyhow::bail!("Only BlockTransactions::Full are supported."),
        }
    }

    /// Stores intermediate trie nodes in the key-value store. Assumes that all nodes passed are
    /// raw, RLP encoded trie nodes.
    async fn store_trie_nodes<T: AsRef<[u8]>>(&self, nodes: &[T]) -> Result<()> {
        let mut hb = kona_mpt::ordered_trie_with_encoder(nodes, |node, buf| {
            buf.put_slice(node.as_ref());
        });
        let intermediates = hb.take_proofs();

        let mut kv_write_lock = self.kv_store.write().await;
        for (_, value) in intermediates.into_iter() {
            let value_hash = keccak256(value.as_ref());
            let key = PreimageKey::new(*value_hash, PreimageKeyType::Keccak256);

            kv_write_lock.set(key.into(), value.into());
        }

        Ok(())
    }
}
