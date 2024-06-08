//! Testing utilities for `kona-mpt`

extern crate std;

use crate::{ordered_trie_with_encoder, TrieDBFetcher};
use alloc::{collections::BTreeMap, vec::Vec};
use alloy_consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom, TxEnvelope, TxType};
use alloy_primitives::{keccak256, Bytes, Log, B256};
use alloy_provider::{network::eip2718::Encodable2718, Provider, ProviderBuilder};
use alloy_rpc_types::BlockTransactions;
use anyhow::{anyhow, Result};
use reqwest::Url;

const RPC_URL: &str = "https://docs-demo.quiknode.pro/";

/// Grabs a live merkleized receipts list within a block header.
pub(crate) async fn get_live_derivable_receipts_list(
) -> Result<(B256, BTreeMap<B256, Bytes>, Vec<ReceiptEnvelope>)> {
    // Initialize the provider.
    let provider = ProviderBuilder::new()
        .on_http(Url::parse(RPC_URL).expect("invalid rpc url"))
        .map_err(|e| anyhow!(e))?;

    let block_number = 19005266;
    let block = provider
        .get_block(block_number.into(), true)
        .await
        .map_err(|e| anyhow!(e))?
        .ok_or(anyhow!("Missing block"))?;
    let receipts = provider
        .get_block_receipts(block_number.into())
        .await
        .map_err(|e| anyhow!(e))?
        .ok_or(anyhow!("Missing receipts"))?;

    let consensus_receipts = receipts
        .into_iter()
        .map(|r| {
            let rpc_receipt = r.inner.as_receipt_with_bloom().expect("Infalliable");
            let consensus_receipt = ReceiptWithBloom::new(
                Receipt {
                    status: rpc_receipt.receipt.status,
                    cumulative_gas_used: rpc_receipt.receipt.cumulative_gas_used,
                    logs: rpc_receipt
                        .receipt
                        .logs
                        .iter()
                        .map(|l| Log { address: l.address(), data: l.data().clone() })
                        .collect(),
                },
                rpc_receipt.logs_bloom,
            );

            match r.transaction_type() {
                TxType::Legacy => ReceiptEnvelope::Legacy(consensus_receipt),
                TxType::Eip2930 => ReceiptEnvelope::Eip2930(consensus_receipt),
                TxType::Eip1559 => ReceiptEnvelope::Eip1559(consensus_receipt),
                TxType::Eip4844 => ReceiptEnvelope::Eip4844(consensus_receipt),
            }
        })
        .collect::<Vec<_>>();

    // Compute the derivable list
    let mut list =
        ordered_trie_with_encoder(consensus_receipts.as_ref(), |rlp, buf| rlp.encode_2718(buf));
    let root = list.root();

    // Sanity check receipts root is correct
    assert_eq!(block.header.receipts_root, root);

    // Construct the mapping of hashed intermediates -> raw intermediates
    let preimages =
        list.take_proofs().into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
            acc.insert(keccak256(value.as_ref()), value);
            acc
        });

    Ok((root, preimages, consensus_receipts))
}

/// Grabs a live merkleized transactions list within a block header.
pub(crate) async fn get_live_derivable_transactions_list(
) -> Result<(B256, BTreeMap<B256, Bytes>, Vec<TxEnvelope>)> {
    // Initialize the provider.
    let provider = ProviderBuilder::new()
        .on_http(Url::parse(RPC_URL).expect("invalid rpc url"))
        .map_err(|e| anyhow!(e))?;

    let block_number = 19005266;
    let block = provider
        .get_block(block_number.into(), true)
        .await
        .map_err(|e| anyhow!(e))?
        .ok_or(anyhow!("Missing block"))?;

    let BlockTransactions::Full(txs) = block.transactions else {
        anyhow::bail!("Did not fetch full block");
    };
    let consensus_txs = txs
        .into_iter()
        .map(|tx| TxEnvelope::try_from(tx).map_err(|e| anyhow!(e)))
        .collect::<Result<Vec<_>>>()?;

    // Compute the derivable list
    let mut list =
        ordered_trie_with_encoder(consensus_txs.as_ref(), |rlp, buf| rlp.encode_2718(buf));
    let root = list.root();

    // Sanity check transaction root is correct
    assert_eq!(block.header.transactions_root, root);

    // Construct the mapping of hashed intermediates -> raw intermediates
    let preimages =
        list.take_proofs().into_iter().fold(BTreeMap::default(), |mut acc, (_, value)| {
            acc.insert(keccak256(value.as_ref()), value);
            acc
        });

    Ok((root, preimages, consensus_txs))
}

/// A mock [TrieDBFetcher] for testing that serves in-memory preimages.
pub(crate) struct TrieNodeProvider {
    preimages: BTreeMap<B256, Bytes>,
    bytecode: BTreeMap<B256, Bytes>,
    headers: BTreeMap<B256, alloy_consensus::Header>,
}

impl TrieNodeProvider {
    pub(crate) fn new(
        preimages: BTreeMap<B256, Bytes>,
        bytecode: BTreeMap<B256, Bytes>,
        headers: BTreeMap<B256, alloy_consensus::Header>,
    ) -> Self {
        Self { preimages, bytecode, headers }
    }
}

impl TrieDBFetcher for TrieNodeProvider {
    fn trie_node_preimage(&self, key: B256) -> Result<Bytes> {
        self.preimages.get(&key).cloned().ok_or_else(|| anyhow!("Key not found"))
    }

    fn bytecode_by_hash(&self, hash: B256) -> Result<Bytes> {
        self.bytecode.get(&hash).cloned().ok_or_else(|| anyhow!("Key not found"))
    }

    fn header_by_hash(&self, hash: B256) -> Result<alloy_consensus::Header> {
        self.headers.get(&hash).cloned().ok_or_else(|| anyhow!("Key not found"))
    }
}
