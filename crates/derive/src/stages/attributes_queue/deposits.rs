//! Contains a helper method to derive deposit transactions from L1 Receipts.

use crate::types::{DepositError, RawTransaction};
use alloc::vec::Vec;
use alloy_consensus::Receipt;
use alloy_primitives::{Address, Log, B256};
use kona_primitives::{decode_deposit, DEPOSIT_EVENT_ABI_HASH};

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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloy_primitives::{address, Bytes, LogData, U256, U64};

    fn generate_valid_log() -> Log {
        let deposit_contract = address!("1111111111111111111111111111111111111111");
        let mut data = vec![0u8; 192];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        // Copy the u128 mint value
        let mint: [u8; 16] = 10_u128.to_be_bytes();
        data[80..96].copy_from_slice(&mint);
        // Copy the tx value
        let value: [u8; 32] = U256::from(100).to_be_bytes();
        data[96..128].copy_from_slice(&value);
        // Copy the gas limit
        let gas: [u8; 8] = 1000_u64.to_be_bytes();
        data[128..136].copy_from_slice(&gas);
        // Copy the isCreation flag
        data[136] = 1;
        let from = address!("2222222222222222222222222222222222222222");
        let mut from_bytes = vec![0u8; 32];
        from_bytes[12..32].copy_from_slice(from.as_slice());
        let to = address!("3333333333333333333333333333333333333333");
        let mut to_bytes = vec![0u8; 32];
        to_bytes[12..32].copy_from_slice(to.as_slice());
        Log {
            address: deposit_contract,
            data: LogData::new_unchecked(
                vec![
                    DEPOSIT_EVENT_ABI_HASH,
                    B256::from_slice(&from_bytes),
                    B256::from_slice(&to_bytes),
                    B256::default(),
                ],
                Bytes::from(data),
            ),
        }
    }

    fn generate_valid_receipt() -> Receipt {
        let mut bad_dest_log = generate_valid_log();
        bad_dest_log.data.topics_mut()[1] = B256::default();
        let mut invalid_topic_log = generate_valid_log();
        invalid_topic_log.data.topics_mut()[0] = B256::default();
        Receipt {
            status: true,
            logs: vec![generate_valid_log(), bad_dest_log, invalid_topic_log],
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_derive_deposits_empty() {
        let receipts = vec![];
        let deposit_contract = Address::default();
        let result = derive_deposits(B256::default(), receipts, deposit_contract).await;
        assert_eq!(result.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn test_derive_deposits_non_deposit_events_filtered_out() {
        let deposit_contract = address!("1111111111111111111111111111111111111111");
        let mut invalid = generate_valid_receipt();
        invalid.logs[0].data = LogData::new_unchecked(vec![], Bytes::default());
        let receipts = vec![generate_valid_receipt(), generate_valid_receipt(), invalid];
        let result = derive_deposits(B256::default(), receipts, deposit_contract).await;
        assert_eq!(result.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_derive_deposits_non_deposit_contract_addr() {
        let deposit_contract = address!("1111111111111111111111111111111111111111");
        let mut invalid = generate_valid_receipt();
        invalid.logs[0].address = Address::default();
        let receipts = vec![generate_valid_receipt(), generate_valid_receipt(), invalid];
        let result = derive_deposits(B256::default(), receipts, deposit_contract).await;
        assert_eq!(result.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn test_derive_deposits_decoding_errors() {
        let deposit_contract = address!("1111111111111111111111111111111111111111");
        let mut invalid = generate_valid_receipt();
        invalid.logs[0].data =
            LogData::new_unchecked(vec![DEPOSIT_EVENT_ABI_HASH], Bytes::default());
        let receipts = vec![generate_valid_receipt(), generate_valid_receipt(), invalid];
        let result = derive_deposits(B256::default(), receipts, deposit_contract).await;
        let downcasted = result.unwrap_err().downcast::<DepositError>().unwrap();
        assert_eq!(downcasted, DepositError::UnexpectedTopicsLen(1));
    }

    #[tokio::test]
    async fn test_derive_deposits_succeeds() {
        let deposit_contract = address!("1111111111111111111111111111111111111111");
        let receipts = vec![generate_valid_receipt(), generate_valid_receipt()];
        let result = derive_deposits(B256::default(), receipts, deposit_contract).await;
        assert_eq!(result.unwrap().len(), 4);
    }
}
