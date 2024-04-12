//! Contains deposit transaction types and helper methods.

use alloc::{string::String, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes, Log, TxKind, B256, U256, U64};
use alloy_rlp::Encodable;
use op_alloy_consensus::TxDeposit;

use crate::{
    params::DEPOSIT_EVENT_ABI_HASH,
    types::{DepositError, RawTransaction},
};

/// Source domain identifiers for deposit transactions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DepositSourceDomainIdentifier {
    /// A user deposit source.
    User = 0,
    /// A L1 info deposit source.
    L1Info = 1,
    /// An upgrade deposit source.
    Upgrade = 2,
}

/// Source domains for deposit transactions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DepositSourceDomain {
    /// A user deposit source.
    User(UserDepositSource),
    /// A L1 info deposit source.
    L1Info(L1InfoDepositSource),
    /// An upgrade deposit source.
    Upgrade(UpgradeDepositSource),
}

/// A deposit transaction source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserDepositSource {
    /// The L1 block hash.
    pub l1_block_hash: B256,
    /// The log index.
    pub log_index: u64,
}

impl UserDepositSource {
    /// Creates a new [UserDepositSource].
    pub fn new(l1_block_hash: B256, log_index: u64) -> Self {
        Self { l1_block_hash, log_index }
    }

    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        let mut input = [0u8; 32 * 2];
        input[..32].copy_from_slice(&self.l1_block_hash[..]);
        input[32 * 2 - 8..].copy_from_slice(&self.log_index.to_be_bytes());
        let deposit_id_hash = keccak256(input);
        let mut domain_input = [0u8; 32 * 2];
        let identifier_bytes: [u8; 8] = (DepositSourceDomainIdentifier::User as u64).to_be_bytes();
        domain_input[32 - 8..32].copy_from_slice(&identifier_bytes);
        domain_input[32..].copy_from_slice(&deposit_id_hash[..]);
        keccak256(domain_input)
    }
}

/// A L1 info deposit transaction source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct L1InfoDepositSource {
    /// The L1 block hash.
    pub l1_block_hash: B256,
    /// The sequence number.
    pub seq_number: u64,
}

impl L1InfoDepositSource {
    /// Creates a new [L1InfoDepositSource].
    pub fn new(l1_block_hash: B256, seq_number: u64) -> Self {
        Self { l1_block_hash, seq_number }
    }

    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        let mut input = [0u8; 32 * 2];
        input[..32].copy_from_slice(&self.l1_block_hash[..]);
        input[32 * 2 - 8..].copy_from_slice(&self.seq_number.to_be_bytes());
        let deposit_id_hash = keccak256(input);
        let mut domain_input = [0u8; 32 * 2];
        let identifier_bytes: [u8; 8] =
            (DepositSourceDomainIdentifier::L1Info as u64).to_be_bytes();
        domain_input[32 - 8..32].copy_from_slice(&identifier_bytes);
        domain_input[32..].copy_from_slice(&deposit_id_hash[..]);
        keccak256(domain_input)
    }
}

/// An upgrade deposit transaction source.
/// This implements the translation of upgrade-tx identity information to a deposit source-hash,
/// which makes the deposit uniquely identifiable.
/// System-upgrade transactions have their own domain for source-hashes,
/// to not conflict with user-deposits or deposited L1 information.
/// The intent identifies the upgrade-tx uniquely, in a human-readable way.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpgradeDepositSource {
    /// The intent.
    pub intent: String,
}

impl UpgradeDepositSource {
    /// Creates a new [UpgradeDepositSource].
    pub fn new(intent: String) -> Self {
        Self { intent }
    }

    /// Returns the source hash.
    pub fn source_hash(&self) -> B256 {
        let intent_hash = keccak256(self.intent.as_bytes());
        let mut domain_input = [0u8; 32 * 2];
        let identifier_bytes: [u8; 8] =
            (DepositSourceDomainIdentifier::Upgrade as u64).to_be_bytes();
        domain_input[32 - 8..32].copy_from_slice(&identifier_bytes);
        domain_input[32..].copy_from_slice(&intent_hash[..]);
        keccak256(domain_input)
    }
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
pub(crate) fn decode_deposit(
    block_hash: B256,
    index: usize,
    log: &Log,
) -> Result<RawTransaction, DepositError> {
    let topics = log.data.topics();
    if topics.len() != 4 {
        return Err(DepositError::UnexpectedTopicsLen(topics.len()));
    }
    if topics[0] != DEPOSIT_EVENT_ABI_HASH {
        return Err(DepositError::InvalidSelector(DEPOSIT_EVENT_ABI_HASH, topics[0]));
    }
    if log.data.data.len() < 64 {
        return Err(DepositError::IncompleteOpaqueData(log.data.data.len()));
    }
    if log.data.data.len() % 32 != 0 {
        return Err(DepositError::UnalignedData(log.data.data.len()));
    }

    let from = Address::try_from(&topics[1].as_slice()[12..])
        .map_err(|_| DepositError::FromDecode(topics[1]))?;
    let to = Address::try_from(&topics[2].as_slice()[12..])
        .map_err(|_| DepositError::ToDecode(topics[2]))?;
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

    let opaque_content_offset: U64 = U64::try_from_be_slice(&log.data.data[24..32]).ok_or(
        DepositError::InvalidOpaqueDataOffset(Bytes::copy_from_slice(&log.data.data[24..32])),
    )?;
    if opaque_content_offset != U64::from(32) {
        return Err(DepositError::InvalidOpaqueDataOffset(Bytes::copy_from_slice(
            &log.data.data[24..32],
        )));
    }

    // The next 32 bytes indicate the length of the opaqueData content.
    let opaque_content_len =
        u64::from_be_bytes(log.data.data[56..64].try_into().map_err(|_| {
            DepositError::InvalidOpaqueDataLength(Bytes::copy_from_slice(&log.data.data[56..64]))
        })?);
    if opaque_content_len as usize > log.data.data.len() - 64 {
        return Err(DepositError::OpaqueDataOverflow(
            opaque_content_len as usize,
            log.data.data.len() - 64,
        ));
    }
    let padded_len = opaque_content_len.checked_add(32).ok_or(DepositError::OpaqueDataOverflow(
        opaque_content_len as usize,
        log.data.data.len() - 64,
    ))?;
    if padded_len as usize <= log.data.data.len() - 64 {
        return Err(DepositError::PaddedOpaqueDataOverflow(
            log.data.data.len() - 64,
            opaque_content_len as usize,
        ));
    }

    // The remaining data is the opaqueData which is tightly packed and then padded to 32 bytes by
    // the EVM.
    let opaque_data = &log.data.data[64..64 + opaque_content_len as usize];
    let source = UserDepositSource::new(block_hash, index as u64);

    let mut deposit_tx = TxDeposit {
        from,
        is_system_transaction: false,
        source_hash: source.source_hash(),
        ..Default::default()
    };

    // Can only handle version 0 for now
    if !version.is_zero() {
        return Err(DepositError::InvalidVersion(version));
    }

    unmarshal_deposit_version0(&mut deposit_tx, to, opaque_data)?;

    // Re-encode the deposit transaction and return as a RawTransaction
    let mut buffer = Vec::<u8>::new();
    deposit_tx.encode(&mut buffer);
    Ok(RawTransaction::from(buffer))
}

/// Unmarshals a deposit transaction from the opaque data.
pub(crate) fn unmarshal_deposit_version0(
    tx: &mut TxDeposit,
    to: Address,
    data: &[u8],
) -> Result<(), DepositError> {
    if data.len() < 32 + 32 + 8 + 1 {
        return Err(DepositError::UnexpectedOpaqueDataLen(data.len()));
    }

    let mut offset = 0;

    let raw_mint: [u8; 16] = data[offset + 16..offset + 32].try_into().map_err(|_| {
        DepositError::MintDecode(Bytes::copy_from_slice(&data[offset + 16..offset + 32]))
    })?;
    let mint = u128::from_be_bytes(raw_mint);

    // 0 mint is represented as nil to skip minting code
    if mint == 0 {
        tx.mint = None;
    } else {
        tx.mint = Some(mint);
    }
    offset += 32;

    // uint256 value
    tx.value = U256::from_be_slice(&data[offset..offset + 32]);
    offset += 32;

    // uint64 gas
    let raw_gas: [u8; 8] = data[offset..offset + 8]
        .try_into()
        .map_err(|_| DepositError::GasDecode(Bytes::copy_from_slice(&data[offset..offset + 8])))?;
    tx.gas_limit = u64::from_be_bytes(raw_gas);
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

    // Remaining bytes fill the data
    tx.input = Bytes::copy_from_slice(&data[offset..offset + tx_data_len]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloy_primitives::{address, b256, hex, LogData};

    #[test]
    fn test_decode_deposit_invalid_topic_len() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(vec![B256::default()], Bytes::default()),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::UnexpectedTopicsLen(1));
    }

    #[test]
    fn test_decode_deposit_invalid_first_topic() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![B256::default(), B256::default(), B256::default(), B256::default()],
                Bytes::default(),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::InvalidSelector(DEPOSIT_EVENT_ABI_HASH, B256::default()));
    }

    #[test]
    fn test_decode_deposit_incomplete_data() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(vec![0u8; 63]),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::IncompleteOpaqueData(63));
    }

    #[test]
    fn test_decode_deposit_unaligned_data() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(vec![0u8; 65]),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::UnalignedData(65));
    }

    #[test]
    #[ignore]
    fn test_decode_deposit_invalid_from() {}

    #[test]
    #[ignore]
    fn test_decode_deposit_invalid_to() {}

    #[test]
    fn test_decode_deposit_invalid_opaque_data_offset() {
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(vec![0u8; 64]),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::InvalidOpaqueDataOffset(Bytes::from(vec![0u8; 8])));
    }

    #[test]
    fn test_decode_deposit_opaque_data_overflow() {
        let mut data = vec![0u8; 128];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        // The first 64 bytes of the data are identifiers so
        // if this test was to be valid, the data length would be 64 not 128.
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(data),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::OpaqueDataOverflow(128, 64));
    }

    #[test]
    fn test_decode_deposit_padded_overflow() {
        let mut data = vec![0u8; 256];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(64).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(data),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::PaddedOpaqueDataOverflow(192, 64));
    }

    #[test]
    fn test_decode_deposit_invalid_version() {
        let mut data = vec![0u8; 128];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(64).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let version = b256!("0000000000000000000000000000000000000000000000000000000000000001");
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), version],
                Bytes::from(data),
            ),
        };
        let err = decode_deposit(B256::default(), 0, &log).unwrap_err();
        assert_eq!(err, DepositError::InvalidVersion(version));
    }

    #[test]
    fn test_decode_deposit_empty_succeeds() {
        let mut data = vec![0u8; 192];
        let offset: [u8; 8] = U64::from(32).to_be_bytes();
        data[24..32].copy_from_slice(&offset);
        let len: [u8; 8] = U64::from(128).to_be_bytes();
        data[56..64].copy_from_slice(&len);
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![DEPOSIT_EVENT_ABI_HASH, B256::default(), B256::default(), B256::default()],
                Bytes::from(data),
            ),
        };
        let tx = decode_deposit(B256::default(), 0, &log).unwrap();
        let raw_hex = hex!("f887a0ed428e1c45e1d9561b62834e1a2d3015a0caae3bfdc16b4da059ac885b01a14594000000000000000000000000000000000000000094000000000000000000000000000000000000000080808080b700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let expected = RawTransaction::from(Bytes::from(raw_hex));
        assert_eq!(tx, expected);
    }

    #[test]
    fn test_decode_deposit_full_succeeds() {
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
        let from = address!("1111111111111111111111111111111111111111");
        let mut from_bytes = vec![0u8; 32];
        from_bytes[12..32].copy_from_slice(from.as_slice());
        let to = address!("2222222222222222222222222222222222222222");
        let mut to_bytes = vec![0u8; 32];
        to_bytes[12..32].copy_from_slice(to.as_slice());
        let log = Log {
            address: Address::default(),
            data: LogData::new_unchecked(
                vec![
                    DEPOSIT_EVENT_ABI_HASH,
                    B256::from_slice(&from_bytes),
                    B256::from_slice(&to_bytes),
                    B256::default(),
                ],
                Bytes::from(data),
            ),
        };
        let tx = decode_deposit(B256::default(), 0, &log).unwrap();
        let raw_hex = hex!("f875a0ed428e1c45e1d9561b62834e1a2d3015a0caae3bfdc16b4da059ac885b01a145941111111111111111111111111111111111111111800a648203e880b700000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");
        let expected = RawTransaction::from(Bytes::from(raw_hex));
        assert_eq!(tx, expected);
    }

    #[test]
    fn test_unmarshal_deposit_version0_invalid_len() {
        let data = vec![0u8; 72];
        let mut tx = TxDeposit::default();
        let to = address!("5555555555555555555555555555555555555555");
        let err = unmarshal_deposit_version0(&mut tx, to, &data).unwrap_err();
        assert_eq!(err, DepositError::UnexpectedOpaqueDataLen(72));

        // Data must have at least length 73
        let data = vec![0u8; 73];
        let mut tx = TxDeposit::default();
        let to = address!("5555555555555555555555555555555555555555");
        unmarshal_deposit_version0(&mut tx, to, &data).unwrap();
    }

    #[test]
    fn test_unmarshal_deposit_version0() {
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
        let mut tx = TxDeposit {
            from: address!("1111111111111111111111111111111111111111"),
            to: TxKind::Call(address!("2222222222222222222222222222222222222222")),
            value: U256::from(100),
            gas_limit: 1000,
            mint: Some(10),
            ..Default::default()
        };
        let to = address!("5555555555555555555555555555555555555555");
        unmarshal_deposit_version0(&mut tx, to, &data).unwrap();
        assert_eq!(tx.to, TxKind::Call(address!("5555555555555555555555555555555555555555")));
    }
}
