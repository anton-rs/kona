//! Contains a helper method to derive deposit transactions from L1 Receipts.

use crate::{params::DEPOSIT_EVENT_ABI_HASH, types::RawTransaction};
use alloc::vec::Vec;
use alloy_consensus::Receipt;
use alloy_primitives::{Address, U64, Log};
use anyhow::{anyhow, Result};

/// Derive deposits for transaction receipts.
///
/// Successful deposits must be emitted by the deposit contract and have the correct event
/// signature. So the receipt address must equal the specified deposit contract and the first topic
/// must be the [DEPOSIT_EVENT_ABI_HASH].
pub(crate) async fn derive_deposits(
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
    logs.iter().map(decode_deposit).collect::<Result<Vec<_>>>()
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
pub(crate) fn decode_deposit(log: &Log) -> Result<RawTransaction> {
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

    let _from = Address::try_from(&topics[1].as_slice()[12..])
        .map_err(|_| anyhow!("Failed to decode `from` address {}", topics[1]))?;
    let _to = Address::try_from(&topics[2].as_slice()[12..])
        .map_err(|_| anyhow!("Failed to decode `to` address {}", topics[2]))?;
    let _version = log.data.topics()[3];

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
    
    let opaque_content_offset: U64 = U64::try_from_be_slice(&log.data.data[24..32]).ok_or(anyhow!(
        "Failed to decode opaqueData slice header offset as U64: {:?}",
        &log.data.data[24..32]
    ))?;
    if opaque_content_offset != U64::from(32) {
        return Err(anyhow!(
            "invalid opaqueData slice header offset: {}",
            opaque_content_offset
        ));
    }

    // The next 32 bytes indicate the length of the opaqueData content.
    let opaque_content_len = U64::try_from_be_slice(&log.data.data[32..64]).ok_or(anyhow!(
        "Failed to decode opaqueData slice header offset as U64: {:?}",
        &log.data.data[24..32]
    ))?;
    if opaque_content_len > U64::from(log.data.data.len() - 64) {
        return Err(anyhow!(
            "opaqueData content length {} exceeds log data length {}",
            opaque_content_len, log.data.data.len() - 64
        ));
    }
    if opaque_content_len + 32 <= U64::from(log.data.data.len() - 64) {
        return Err(anyhow!(
            "opaqueData data with possible padding {} exceeds specified content length {}",
            log.data.data.len() - 64, opaque_content_len
        ));
    }

    // The remaining data is the opaqueData which is tightly packed and then padded to 32 bytes by the EVM.
    let opaque_data = &log.data.data[64..64 + opaque_content_len as usize];

    // TODO: construct the deposit transaction using the opaque data

    // TODO: decode the log data into a deposit tx
    // TODO: re-encode the deposit transaction into bytes that can be turned into a RawTransaction
    let encoded = Vec::new();
    Ok(RawTransaction::from(encoded))
}


	// // The remaining data is the opaqueData which is tightly packed
	// // and then padded to 32 bytes by the EVM.
	// opaqueData := ev.Data[64 : 64+opaqueContentLength.Uint64()]
	//
	// var dep types.DepositTx
	//
	// source := UserDepositSource{
	// 	L1BlockHash: ev.BlockHash,
	// 	LogIndex:    uint64(ev.Index),
	// }
	// dep.SourceHash = source.SourceHash()
	// dep.From = from
	// dep.IsSystemTransaction = false
	//
	// var err error
	// switch version {
	// case DepositEventVersion0:
	// 	err = unmarshalDepositVersion0(&dep, to, opaqueData)
	// default:
	// 	return nil, fmt.Errorf("invalid deposit version, got %s", version)
	// }
	// if err != nil {
	// 	return nil, fmt.Errorf("failed to decode deposit (version %s): %w", version, err)
	// }
	// return &dep, nil
	//


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
