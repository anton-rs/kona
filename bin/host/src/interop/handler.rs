//! [HintHandler] for the [InteropHost].

use super::InteropHost;
use crate::{
    backend::util::store_ordered_trie, HintHandler, OnlineHostBackendCfg, SharedKeyValueStore,
};
use alloy_consensus::Header;
use alloy_eips::{
    eip2718::Encodable2718,
    eip4844::{IndexedBlobHash, FIELD_ELEMENTS_PER_BLOB},
};
use alloy_primitives::{address, keccak256, Address, Bytes, B256};
use alloy_provider::Provider;
use alloy_rlp::Decodable;
use alloy_rpc_types::{Block, BlockTransactionsKind};
use anyhow::{anyhow, ensure, Result};
use async_trait::async_trait;
use kona_preimage::{PreimageKey, PreimageKeyType};
use kona_proof_interop::{HintType, PreState};
use maili_protocol::BlockInfo;
use maili_registry::ROLLUP_CONFIGS;

/// The [HintHandler] for the [InteropHost].
#[derive(Debug, Clone, Copy)]
pub struct InteropHintHandler;

#[async_trait]
impl HintHandler for InteropHintHandler {
    type Cfg = InteropHost;

    async fn fetch_hint(
        hint: <Self::Cfg as OnlineHostBackendCfg>::Hint,
        cfg: &Self::Cfg,
        providers: &<Self::Cfg as OnlineHostBackendCfg>::Providers,
        kv: SharedKeyValueStore,
    ) -> Result<()> {
        match hint.hint_type {
            HintType::L1BlockHeader => {
                ensure!(hint.hint_data.len() == 32, "Invalid hint data length");

                let hash: B256 = hint.hint_data.as_ref().try_into()?;
                let raw_header: Bytes =
                    providers.l1.client().request("debug_getRawHeader", [hash]).await?;

                let mut kv_lock = kv.write().await;
                kv_lock.set(PreimageKey::new_keccak256(*hash).into(), raw_header.into())?;
            }
            HintType::L1Transactions => {
                ensure!(hint.hint_data.len() == 32, "Invalid hint data length");

                let hash: B256 = hint.hint_data.as_ref().try_into()?;
                let Block { transactions, .. } = providers
                    .l1
                    .get_block_by_hash(hash, BlockTransactionsKind::Full)
                    .await?
                    .ok_or(anyhow!("Block not found"))?;
                let encoded_transactions = transactions
                    .into_transactions()
                    .map(|tx| tx.inner.encoded_2718())
                    .collect::<Vec<_>>();

                store_ordered_trie(kv.as_ref(), encoded_transactions.as_slice()).await?;
            }
            HintType::L1Receipts => {
                ensure!(hint.hint_data.len() == 32, "Invalid hint data length");

                let hash: B256 = hint.hint_data.as_ref().try_into()?;
                let raw_receipts: Vec<Bytes> =
                    providers.l1.client().request("debug_getRawReceipts", [hash]).await?;

                store_ordered_trie(kv.as_ref(), raw_receipts.as_slice()).await?;
            }
            HintType::L1Blob => {
                ensure!(hint.hint_data.len() == 48, "Invalid hint data length");

                let hash_data_bytes: [u8; 32] = hint.hint_data[0..32].try_into()?;
                let index_data_bytes: [u8; 8] = hint.hint_data[32..40].try_into()?;
                let timestamp_data_bytes: [u8; 8] = hint.hint_data[40..48].try_into()?;

                let hash: B256 = hash_data_bytes.into();
                let index = u64::from_be_bytes(index_data_bytes);
                let timestamp = u64::from_be_bytes(timestamp_data_bytes);

                let partial_block_ref = BlockInfo { timestamp, ..Default::default() };
                let indexed_hash = IndexedBlobHash { index, hash };

                // Fetch the blob sidecar from the blob provider.
                let mut sidecars = providers
                    .blobs
                    .fetch_filtered_sidecars(&partial_block_ref, &[indexed_hash])
                    .await
                    .map_err(|e| anyhow!("Failed to fetch blob sidecars: {e}"))?;
                if sidecars.len() != 1 {
                    anyhow::bail!("Expected 1 sidecar, got {}", sidecars.len());
                }
                let sidecar = sidecars.remove(0);

                // Acquire a lock on the key-value store and set the preimages.
                let mut kv_lock = kv.write().await;

                // Set the preimage for the blob commitment.
                kv_lock.set(
                    PreimageKey::new(*hash, PreimageKeyType::Sha256).into(),
                    sidecar.kzg_commitment.to_vec(),
                )?;

                // Write all the field elements to the key-value store. There should be 4096.
                // The preimage oracle key for each field element is the keccak256 hash of
                // `abi.encodePacked(sidecar.KZGCommitment, uint256(i))`
                let mut blob_key = [0u8; 80];
                blob_key[..48].copy_from_slice(sidecar.kzg_commitment.as_ref());
                for i in 0..FIELD_ELEMENTS_PER_BLOB {
                    blob_key[72..].copy_from_slice(i.to_be_bytes().as_ref());
                    let blob_key_hash = keccak256(blob_key.as_ref());

                    kv_lock
                        .set(PreimageKey::new_keccak256(*blob_key_hash).into(), blob_key.into())?;
                    kv_lock.set(
                        PreimageKey::new(*blob_key_hash, PreimageKeyType::Blob).into(),
                        sidecar.blob[(i as usize) << 5..(i as usize + 1) << 5].to_vec(),
                    )?;
                }

                // Write the KZG Proof as the 4096th element.
                blob_key[72..].copy_from_slice((FIELD_ELEMENTS_PER_BLOB).to_be_bytes().as_ref());
                let blob_key_hash = keccak256(blob_key.as_ref());

                kv_lock.set(PreimageKey::new_keccak256(*blob_key_hash).into(), blob_key.into())?;
                kv_lock.set(
                    PreimageKey::new(*blob_key_hash, PreimageKeyType::Blob).into(),
                    sidecar.kzg_proof.to_vec(),
                )?;
            }
            HintType::L1Precompile => {
                ensure!(hint.hint_data.len() >= 20, "Invalid hint data length");

                let address = Address::from_slice(&hint.hint_data.as_ref()[..20]);
                let input = hint.hint_data[20..].to_vec();
                let input_hash = keccak256(hint.hint_data.as_ref());

                let result = crate::eth::execute(address, input).map_or_else(
                    |_| vec![0u8; 1],
                    |raw_res| {
                        let mut res = Vec::with_capacity(1 + raw_res.len());
                        res.push(0x01);
                        res.extend_from_slice(&raw_res);
                        res
                    },
                );

                let mut kv_lock = kv.write().await;
                kv_lock
                    .set(PreimageKey::new_keccak256(*input_hash).into(), hint.hint_data.into())?;
                kv_lock.set(
                    PreimageKey::new(*input_hash, PreimageKeyType::Precompile).into(),
                    result,
                )?;
            }
            HintType::AgreedPreState => {
                ensure!(hint.hint_data.len() == 32, "Invalid hint data length");

                let hash: B256 = hint.hint_data.as_ref().try_into()?;

                if hash != keccak256(cfg.agreed_l2_pre_state.as_ref()) {
                    anyhow::bail!("Agreed pre-state hash does not match.");
                }

                let mut kv_write_lock = kv.write().await;
                kv_write_lock.set(
                    PreimageKey::new_keccak256(*hash).into(),
                    cfg.agreed_l2_pre_state.clone().into(),
                )?;
            }
            HintType::L2OutputRoot => {
                const OUTPUT_ROOT_VERSION: u8 = 0;
                const L2_TO_L1_MESSAGE_PASSER_ADDRESS: Address =
                    address!("4200000000000000000000000000000000000016");

                ensure!(
                    hint.hint_data.len() >= 32 && hint.hint_data.len() <= 40,
                    "Invalid hint data length"
                );

                let hash = B256::from_slice(&hint.hint_data.as_ref()[0..32]);
                let chain_id = u64::from_be_bytes(hint.hint_data.as_ref()[32..40].try_into()?);
                let l2_provider = providers.l2(&chain_id)?;

                // Decode the pre-state to determine the timestamp of the block.
                let pre = PreState::decode(&mut cfg.agreed_l2_pre_state.as_ref())?;
                let timestamp = match pre {
                    PreState::SuperRoot(super_root) => super_root.timestamp,
                    PreState::TransitionState(transition_state) => {
                        transition_state.pre_state.timestamp
                    }
                };

                // Convert the timestamp to an L2 block number, using the rollup config for the
                // chain ID embedded within the hint.
                let rollup_config = ROLLUP_CONFIGS
                    .get(&chain_id)
                    .cloned()
                    .or_else(|| {
                        let local_cfgs = cfg.read_rollup_configs().ok()?;
                        local_cfgs.get(&chain_id).cloned()
                    })
                    .ok_or(anyhow!("No rollup config found for chain ID: {chain_id}"))?;
                let block_number =
                    (timestamp - rollup_config.genesis.l2_time) / rollup_config.block_time;

                // Fetch the header for the L2 head block.
                let raw_header: Bytes = l2_provider
                    .client()
                    .request("debug_getRawHeader", &[format!("0x{block_number:x}")])
                    .await
                    .map_err(|e| anyhow!("Failed to fetch header RLP: {e}"))?;
                let header = Header::decode(&mut raw_header.as_ref())?;

                // Fetch the storage root for the L2 head block.
                let l2_to_l1_message_passer = l2_provider
                    .get_proof(L2_TO_L1_MESSAGE_PASSER_ADDRESS, Default::default())
                    .block_id(block_number.into())
                    .await?;

                let mut raw_output = [0u8; 128];
                raw_output[31] = OUTPUT_ROOT_VERSION;
                raw_output[32..64].copy_from_slice(header.state_root.as_ref());
                raw_output[64..96].copy_from_slice(l2_to_l1_message_passer.storage_hash.as_ref());
                raw_output[96..128].copy_from_slice(header.hash_slow().as_ref());
                let output_root = keccak256(raw_output);

                ensure!(
                    output_root == hash,
                    "Output root does not match L2 head. Expected: {hash}, got: {output_root}"
                );

                let mut kv_lock = kv.write().await;
                kv_lock.set(PreimageKey::new_keccak256(*output_root).into(), raw_output.into())?;
            }
            HintType::L2BlockHeader => {
                ensure!(
                    hint.hint_data.len() >= 32 && hint.hint_data.len() <= 40,
                    "Invalid hint data length"
                );

                let hash: B256 = hint.hint_data.as_ref()[..32].try_into()?;
                let chain_id = if hint.hint_data.len() == 40 {
                    u64::from_be_bytes(hint.hint_data[32..40].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };

                let raw_header: Bytes =
                    providers.l2(&chain_id)?.client().request("debug_getRawHeader", [hash]).await?;

                let mut kv_lock = kv.write().await;
                kv_lock.set(PreimageKey::new_keccak256(*hash).into(), raw_header.into())?;
            }
            HintType::L2Transactions => {
                ensure!(
                    hint.hint_data.len() >= 32 && hint.hint_data.len() <= 40,
                    "Invalid hint data length"
                );

                let hash: B256 = hint.hint_data.as_ref()[..32].try_into()?;
                let chain_id = if hint.hint_data.len() == 40 {
                    u64::from_be_bytes(hint.hint_data[32..40].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };

                let Block { transactions, .. } = providers
                    .l2(&chain_id)?
                    .get_block_by_hash(hash, BlockTransactionsKind::Full)
                    .await?
                    .ok_or(anyhow!("Block not found"))?;
                let encoded_transactions = transactions
                    .into_transactions()
                    .map(|tx| tx.inner.inner.encoded_2718())
                    .collect::<Vec<_>>();

                store_ordered_trie(kv.as_ref(), encoded_transactions.as_slice()).await?;
            }
            HintType::L2Receipts => {
                ensure!(
                    hint.hint_data.len() >= 32 && hint.hint_data.len() <= 40,
                    "Invalid hint data length"
                );

                let hash: B256 = hint.hint_data.as_ref()[..32].try_into()?;
                let chain_id = if hint.hint_data.len() == 40 {
                    u64::from_be_bytes(hint.hint_data[32..40].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };

                let raw_receipts: Vec<Bytes> = providers
                    .l2(&chain_id)?
                    .client()
                    .request("debug_getRawReceipts", [hash])
                    .await?;

                store_ordered_trie(kv.as_ref(), raw_receipts.as_slice()).await?;
            }
            HintType::L2Code => {
                // geth hashdb scheme code hash key prefix
                const CODE_PREFIX: u8 = b'c';

                ensure!(
                    hint.hint_data.len() >= 32 && hint.hint_data.len() <= 40,
                    "Invalid hint data length"
                );

                let hash: B256 = hint.hint_data[..32].as_ref().try_into()?;
                let chain_id = if hint.hint_data.len() == 40 {
                    u64::from_be_bytes(hint.hint_data[32..40].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };
                let l2_provider = providers.l2(&chain_id)?;

                // Attempt to fetch the code from the L2 chain provider.
                let code_key = [&[CODE_PREFIX], hash.as_slice()].concat();
                let code = l2_provider
                    .client()
                    .request::<&[Bytes; 1], Bytes>("debug_dbGet", &[code_key.into()])
                    .await;

                // Check if the first attempt to fetch the code failed. If it did, try fetching the
                // code hash preimage without the geth hashdb scheme prefix.
                let code = match code {
                    Ok(code) => code,
                    Err(_) => l2_provider
                        .client()
                        .request::<&[B256; 1], Bytes>("debug_dbGet", &[hash])
                        .await
                        .map_err(|e| anyhow!("Error fetching code hash preimage: {e}"))?,
                };

                let mut kv_lock = kv.write().await;
                kv_lock.set(PreimageKey::new_keccak256(*hash).into(), code.into())?;
            }
            HintType::L2StateNode => {
                ensure!(
                    hint.hint_data.len() >= 32 && hint.hint_data.len() <= 40,
                    "Invalid hint data length"
                );

                let hash: B256 = hint.hint_data.as_ref().try_into()?;
                let chain_id = if hint.hint_data.len() == 40 {
                    u64::from_be_bytes(hint.hint_data[32..40].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };

                // Fetch the preimage from the L2 chain provider.
                let preimage: Bytes =
                    providers.l2(&chain_id)?.client().request("debug_dbGet", &[hash]).await?;

                let mut kv_write_lock = kv.write().await;
                kv_write_lock.set(PreimageKey::new_keccak256(*hash).into(), preimage.into())?;
            }
            HintType::L2AccountProof => {
                ensure!(
                    hint.hint_data.len() >= 8 + 20 && hint.hint_data.len() <= 8 + 20 + 8,
                    "Invalid hint data length"
                );

                let block_number = u64::from_be_bytes(hint.hint_data.as_ref()[..8].try_into()?);
                let address = Address::from_slice(&hint.hint_data.as_ref()[8..28]);
                let chain_id = if hint.hint_data.len() == 8 + 20 + 8 {
                    u64::from_be_bytes(hint.hint_data[28..].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };

                let proof_response = providers
                    .l2(&chain_id)?
                    .get_proof(address, Default::default())
                    .block_id(block_number.into())
                    .await?;

                // Write the account proof nodes to the key-value store.
                let mut kv_lock = kv.write().await;
                proof_response.account_proof.into_iter().try_for_each(|node| {
                    let node_hash = keccak256(node.as_ref());
                    let key = PreimageKey::new_keccak256(*node_hash);
                    kv_lock.set(key.into(), node.into())?;
                    Ok::<(), anyhow::Error>(())
                })?;
            }
            HintType::L2AccountStorageProof => {
                ensure!(
                    hint.hint_data.len() >= 8 + 20 + 32 && hint.hint_data.len() <= 8 + 20 + 32 + 8,
                    "Invalid hint data length"
                );

                let block_number = u64::from_be_bytes(hint.hint_data.as_ref()[..8].try_into()?);
                let address = Address::from_slice(&hint.hint_data.as_ref()[8..28]);
                let slot = B256::from_slice(&hint.hint_data.as_ref()[28..]);
                let chain_id = if hint.hint_data.len() == 8 + 20 + 32 + 8 {
                    u64::from_be_bytes(hint.hint_data[60..].try_into()?)
                } else {
                    cfg.active_l2_chain_id()?
                };

                let mut proof_response = providers
                    .l2(&chain_id)?
                    .get_proof(address, vec![slot])
                    .block_id(block_number.into())
                    .await?;

                let mut kv_lock = kv.write().await;

                // Write the account proof nodes to the key-value store.
                proof_response.account_proof.into_iter().try_for_each(|node| {
                    let node_hash = keccak256(node.as_ref());
                    let key = PreimageKey::new_keccak256(*node_hash);
                    kv_lock.set(key.into(), node.into())?;
                    Ok::<(), anyhow::Error>(())
                })?;

                // Write the storage proof nodes to the key-value store.
                let storage_proof = proof_response.storage_proof.remove(0);
                storage_proof.proof.into_iter().try_for_each(|node| {
                    let node_hash = keccak256(node.as_ref());
                    let key = PreimageKey::new_keccak256(*node_hash);
                    kv_lock.set(key.into(), node.into())?;
                    Ok::<(), anyhow::Error>(())
                })?;
            }
            HintType::L2BlockData => {
                unimplemented!("L2BlockData hint type is not yet implemented");
            }
        }

        Ok(())
    }
}
