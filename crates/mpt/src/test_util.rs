//! Testing utilities for `kona-mpt`

extern crate std;

use std::dbg;

use alloc::vec::Vec;
use alloy_consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom, TxType};
use alloy_primitives::{Bytes, Log};
use alloy_provider::{network::eip2718::Encodable2718, Provider, ProviderBuilder};
use alloy_rlp::{BufMut, BytesMut, Encodable};
use alloy_trie::{HashBuilder, Nibbles};
use anyhow::{anyhow, Result};
use reqwest::Url;

pub(crate) async fn get_live_derivable_receipts_list() -> Result<()> {
    // Initialize the provider.
    let rpc_url = "http://anton.clab.by:8546";
    let provider = ProviderBuilder::new()
        .on_http(Url::parse(rpc_url).expect("invalid rpc url"))
        .map_err(|e| anyhow!(e))?;

    let block_number = 2000002;
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

    let receipt_rlp = receipts
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

            let envelope = match r.transaction_type() {
                TxType::Legacy => ReceiptEnvelope::Legacy(consensus_receipt),
                TxType::Eip2930 => ReceiptEnvelope::Eip2930(consensus_receipt),
                TxType::Eip1559 => ReceiptEnvelope::Eip1559(consensus_receipt),
                TxType::Eip4844 => ReceiptEnvelope::Eip4844(consensus_receipt),
            };
            let mut rlp_buf = Vec::with_capacity(envelope.length());
            envelope.encode_2718(&mut rlp_buf);
            rlp_buf.into()
        })
        .collect::<Vec<_>>();

    dbg!(&receipt_rlp);
    let mut derivable_list = construct_derivable_list(&receipt_rlp);
    let root = derivable_list.root();

    assert_eq!(block.header.receipts_root, root);

    Ok(())
}

/// Constructs a derivable list from an ordered list of leaves
pub(crate) fn construct_derivable_list(items: &[Bytes]) -> HashBuilder {
    let mut index_buffer = BytesMut::new();
    let mut value_buffer = BytesMut::new();

    let mut hb = HashBuilder::default();
    let items_len = items.len();
    for (i, item) in items.iter().enumerate() {
        let index = adjust_index_for_rlp(i, items_len);

        index_buffer.clear();
        dbg!(index);
        index.encode(&mut index_buffer);

        value_buffer.clear();
        value_buffer.put_slice(item.as_ref());

        hb.add_leaf(Nibbles::unpack(&index_buffer), &value_buffer);
    }

    hb
}

/// Adjust the index of an item for rlp encoding.
pub(crate) const fn adjust_index_for_rlp(i: usize, len: usize) -> usize {
    if i > 0x7f {
        i
    } else if i == 0x7f || i + 1 == len {
        0
    } else {
        i + 1
    }
}

#[tokio::test]
#[ignore]
async fn test_receipt() {
    get_live_derivable_receipts_list().await.unwrap()
}
