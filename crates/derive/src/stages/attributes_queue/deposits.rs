//! Contains a helper method to derive deposit transactions from L1 Receipts.

use crate::{
    params::DEPOSIT_EVENT_ABI_HASH,
    types::{decode_deposit, DepositError, RawTransaction},
};
use alloc::vec::Vec;
use alloy_consensus::Receipt;
use alloy_primitives::{Address, Log, B256};

/// Derive deposits for transaction receipts.
///
/// Successful deposits must be emitted by the deposit contract and have the correct event
/// signature. So the receipt address must equal the specified deposit contract and the first topic
/// must be the [DEPOSIT_EVENT_ABI_HASH].
pub(crate) async fn derive_deposits(
    block_hash: B256,
    receipts: Vec<Receipt>,
    deposit_contract: Address,
) -> anyhow::Result<Vec<RawTransaction>> {
    let receipts = receipts.into_iter().filter(|r| r.status).collect::<Vec<_>>();
    // Flatten the list of receipts into a list of logs.
    let addr = |l: &Log| l.address == deposit_contract;
    let topics = |l: &Log| l.data.topics().first().map_or(false, |i| *i == DEPOSIT_EVENT_ABI_HASH);
    let filter_logs =
        |r: Receipt| r.logs.into_iter().filter(|l| addr(l) && topics(l)).collect::<Vec<Log>>();
    let logs = receipts.into_iter().flat_map(filter_logs).collect::<Vec<Log>>();
    // TODO(refcell): are logs **and** receipts guaranteed to be _in order_?
    //                If not, we need to somehow get the index of each log in the block.
    logs.iter()
        .enumerate()
        .map(|(i, l)| decode_deposit(block_hash, i, l))
        .collect::<Result<Vec<_>, DepositError>>()
        .map_err(|e| anyhow::anyhow!(e))
}
