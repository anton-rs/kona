//! Contains a helper method to derive deposit transactions from L1 Receipts.

use crate::types::UserDepositSource;
use crate::{params::DEPOSIT_EVENT_ABI_HASH, types::RawTransaction};
use alloc::vec::Vec;
use alloy_consensus::Receipt;
use alloy_rlp::Encodable;
use alloy_primitives::{Address, TxKind, B256, U256, Log, U64};
use op_alloy_consensus::TxDeposit;
use anyhow::{anyhow, Result};

/// Derive deposits for transaction receipts.
///
/// Successful deposits must be emitted by the deposit contract and have the correct event
/// signature. So the receipt address must equal the specified deposit contract and the first topic
/// must be the [DEPOSIT_EVENT_ABI_HASH].
pub(crate) async fn derive_deposits(
    block_hash: B256,
    receipts: Vec<Receipt>,
    deposit_contract: Address,
) -> Result<Vec<RawTransaction>> {
    let receipts = receipts.into_iter().filter(|r| r.status).collect::<Vec<_>>();
    // Flatten the list of receipts into a list of logs.
    let addr = |l: &Log| l.address == deposit_contract;
    let topics = |l: &Log| l.data.topics().first().map_or(false, |i| *i == DEPOSIT_EVENT_ABI_HASH);
    let filter_logs =
        |r: Receipt| r.logs.into_iter().filter(|l| addr(l) && topics(l)).collect::<Vec<Log>>();
    let logs = receipts.into_iter().flat_map(filter_logs).collect::<Vec<Log>>();
    // TODO(refcell): are logs **and** receipts guaranteed to be _in order_?
    //                If not, we need to somehow get the index of each log in the block.
    logs.iter().enumerate().map(|(i, l)| decode_deposit(block_hash, i, l)).collect::<Result<Vec<_>>>()
}

/// Derives a deposit transaction from an EVM log event emitted by the deposit contract.
///
/// The emitted log must be in format:
/// ```solidity
/// event TransactionDeposited(
///    address indexed from,
///    address indexed to,
///    uint256 indexed version,
///    bytes opaqueData
/// );
/// ```
pub(crate) fn decode_deposit(block_hash: B256, index: usize, log: &Log) -> Result<RawTransaction> {
    let topics = log.data.topics();
    if topics.len() != 4 {
        return Err(anyhow!("expected 4 event topics, got {}", topics.len()));
    }
    if topics[0] != DEPOSIT_EVENT_ABI_HASH {
        return Err(anyhow!(
            "invalid deposit event selector: {}, expected {}",
            topics[0],
            DEPOSIT_EVENT_ABI_HASH
        ));
    }
    if log.data.data.len() < 64 {
        return Err(anyhow!("incomplete opaqueData slice header: {}", log.data.data.len()));
    }
    if log.data.data.len() % 32 != 0 {
        return Err(anyhow!(
            "expected log data to be multiple of 32 bytes: got {}",
            log.data.data.len()
        ));
    }

    let from = Address::try_from(&topics[1].as_slice()[12..])
        .map_err(|_| anyhow!("Failed to decode `from` address {}", topics[1]))?;
    let to = Address::try_from(&topics[2].as_slice()[12..])
        .map_err(|_| anyhow!("Failed to decode `to` address {}", topics[2]))?;
    let version = log.data.topics()[3];

    // Solidity serializes the event's Data field as follows:
    //
    // ```solidity
    // abi.encode(abi.encodPacked(uint256 mint, uint256 value, uint64 gasLimit, uint8 isCreation, bytes data))
    // ```
    //
    // The the opaqueData will be packed as shown below:
    //
    // ------------------------------------------------------------
    // | offset | 256 byte content                                |
    // ------------------------------------------------------------
    // | 0      | [0; 24] . {U64 big endian, hex encoded offset}  |
    // ------------------------------------------------------------
    // | 32     | [0; 24] . {U64 big endian, hex encoded length}  |
    // ------------------------------------------------------------

    let opaque_content_offset: U64 =
        U64::try_from_be_slice(&log.data.data[24..32]).ok_or(anyhow!(
            "Failed to decode opaqueData slice header offset as U64: {:?}",
            &log.data.data[24..32]
        ))?;
    if opaque_content_offset != U64::from(32) {
        return Err(anyhow!("invalid opaqueData slice header offset: {}", opaque_content_offset));
    }

    // The next 32 bytes indicate the length of the opaqueData content.
    let opaque_content_len =
        u64::from_be_bytes(log.data.data[32..40].try_into().map_err(|_| {
            anyhow!(
                "Failed to decode opaqueData slice header offset as U64: {:?}",
                &log.data.data[24..32]
            )
        })?);
    // let opaque_content_len = U64::try_from_be_slice(&log.data.data[56..64]).ok_or(anyhow!(
    //     "Failed to decode opaqueData slice header offset as U64: {:?}",
    //     &log.data.data[24..32]
    // ))?;
    if opaque_content_len as usize > log.data.data.len() - 64 {
        return Err(anyhow!(
            "opaqueData content length {} exceeds log data length {}",
            opaque_content_len,
            log.data.data.len() - 64
        ));
    }
    let padded_len = opaque_content_len.checked_add(32).ok_or(anyhow!(
        "overflow when adding 32 bytes to opaqueData content length {}",
        opaque_content_len
    ))?;
    if padded_len as usize <= log.data.data.len() - 64 {
        return Err(anyhow!(
            "opaqueData data with possible padding {} exceeds specified content length {}",
            log.data.data.len() - 64,
            opaque_content_len
        ));
    }

    // The remaining data is the opaqueData which is tightly packed and then padded to 32 bytes by
    // the EVM.
    let opaque_data = &log.data.data[64..64 + opaque_content_len as usize];
    let source = UserDepositSource::new(block_hash, index as u64);

    let mut deposit_tx = TxDeposit::default();
    deposit_tx.from = from;
    deposit_tx.is_system_transaction = false;
    deposit_tx.source_hash = source.source_hash();

    // Can only handle version 0 for now
    if !version.is_zero() {
        return Err(anyhow!("invalid deposit version, got {}", version));
    }

    unmarshal_deposit_version0(&mut deposit_tx, to, opaque_data)?;

    // Re-encode the deposit transaction and return as a RawTransaction
    let mut buffer = Vec::<u8>::new();
    deposit_tx.encode(&mut buffer);
    Ok(RawTransaction::from(buffer))
}

/// Unmarshals a deposit transaction from the opaque data.
pub(crate) fn unmarshal_deposit_version0(tx: &mut TxDeposit, to: Address, data: &[u8]) -> Result<()> {
    if data.len() < 32 + 32 + 8 + 1 {
        return Err(anyhow!("unexpected opaqueData length: {}", data.len()));
    }

    let mut offset = 0;

    // uint256 mint
    let mint = u128::from_be_bytes(&data[offset..offset + 32]);
    // 0 mint is represented as nil to skip minting code
    if mint == 0 {
        tx.mint = None;
    } else {
        tx.mint = Some(mint);
    }
    offset += 32;

    // uint256 value
    tx.value = U256::from_be_bytes(&data[offset..offset + 32]);
    offset += 32;

    // uint64 gas
    let gas = U64::from_be_bytes(&data[offset..offset + 8]);
    if gas > u64::MAX {
        return Err(anyhow!("bad gas value: {:?}", &data[offset..offset + 8]));
    }
    tx.gas_limit = gas;
    offset += 8;

    // uint8 isCreation
    // isCreation: If the boolean byte is 1 then dep.To will stay nil,
    // and it will create a contract using L2 account nonce to determine the created address.
    if data[offset] == 0 {
        tx.to = TxKind::Call(to);
    } else {
        tx.to = TxKind::Create;
    }
    offset += 1;

    // The remainder of the opaqueData is the transaction data (without length prefix).
    // The data may be padded to a multiple of 32 bytes
    let tx_data_len = data.len() - offset;

    // remaining bytes fill the data
    tx.data = data[offset..offset + tx_data_len].to_vec();

    Ok(())
}


// func unmarshalDepositVersion0(dep *types.DepositTx, to common.Address, opaqueData []byte) error {
// 	if len(opaqueData) < 32+32+8+1 {
// 		return fmt.Errorf("unexpected opaqueData length: %d", len(opaqueData))
// 	}
// 	offset := uint64(0)
//
// 	// uint256 mint
// 	dep.Mint = new(big.Int).SetBytes(opaqueData[offset : offset+32])
// 	// 0 mint is represented as nil to skip minting code
// 	if dep.Mint.Cmp(new(big.Int)) == 0 {
// 		dep.Mint = nil
// 	}
// 	offset += 32
//
// 	// uint256 value
// 	dep.Value = new(big.Int).SetBytes(opaqueData[offset : offset+32])
// 	offset += 32
//
// 	// uint64 gas
// 	gas := new(big.Int).SetBytes(opaqueData[offset : offset+8])
// 	if !gas.IsUint64() {
// 		return fmt.Errorf("bad gas value: %x", opaqueData[offset:offset+8])
// 	}
// 	dep.Gas = gas.Uint64()
// 	offset += 8
//
// 	// uint8 isCreation
// 	// isCreation: If the boolean byte is 1 then dep.To will stay nil,
// 	// and it will create a contract using L2 account nonce to determine the created address.
// 	if opaqueData[offset] == 0 {
// 		dep.To = &to
// 	}
// 	offset += 1
//
// 	// The remainder of the opaqueData is the transaction data (without length prefix).
// 	// The data may be padded to a multiple of 32 bytes
// 	txDataLen := uint64(len(opaqueData)) - offset
//
// 	// remaining bytes fill the data
// 	dep.Data = opaqueData[offset : offset+txDataLen]
//
// 	return nil
// }
