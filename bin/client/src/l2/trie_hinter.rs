//! Contains the hinter for the [TrieDB].
//!
//! [TrieDB]: kona_mpt::TrieDB

use crate::{HintType, HINT_WRITER};
use alloy_primitives::{Address, B256};
use anyhow::Result;
use kona_mpt::TrieDBHinter;
use kona_preimage::HintWriterClient;

/// The [TrieDBHinter] implementation for the block executor's [TrieDB].
///
/// [TrieDB]: kona_mpt::TrieDB
#[derive(Debug)]
pub struct TrieDBHintWriter;

impl TrieDBHinter for TrieDBHintWriter {
    fn hint_trie_node(&self, hash: B256) -> Result<()> {
        kona_common::block_on(async move {
            HINT_WRITER.write(&HintType::L2StateNode.encode_with(&[hash.as_slice()])).await
        })
    }

    fn hint_account_proof(&self, address: Address, block_number: u64) -> Result<()> {
        kona_common::block_on(async move {
            HINT_WRITER
                .write(
                    &HintType::L2AccountProof
                        .encode_with(&[block_number.to_be_bytes().as_ref(), address.as_slice()]),
                )
                .await
        })
    }

    fn hint_storage_proof(
        &self,
        address: alloy_primitives::Address,
        slot: alloy_primitives::U256,
        block_number: u64,
    ) -> Result<()> {
        kona_common::block_on(async move {
            HINT_WRITER
                .write(&HintType::L2AccountStorageProof.encode_with(&[
                    block_number.to_be_bytes().as_ref(),
                    address.as_slice(),
                    slot.to_be_bytes::<32>().as_ref(),
                ]))
                .await
        })
    }
}
