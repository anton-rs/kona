//! Contains deposit transaction types and helper methods.

use alloc::string::String;
use alloy_primitives::{keccak256, B256};

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
        let deposit_id_hash = keccak256(&input);
        let mut domain_input = [0u8; 32 * 2];
        domain_input[32 - 8..32].copy_from_slice(&[DepositSourceDomainIdentifier::User as u8]);
        domain_input[32..].copy_from_slice(&deposit_id_hash[..]);
        keccak256(&domain_input)
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
        let deposit_id_hash = keccak256(&input);
        let mut domain_input = [0u8; 32 * 2];
        domain_input[32 - 8..32].copy_from_slice(&[DepositSourceDomain::L1Info as u8]);
        domain_input[32..].copy_from_slice(&deposit_id_hash[..]);
        keccak256(&domain_input)
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
        domain_input[32 - 8..32].copy_from_slice(&[DepositSourceDomain::Upgrade as u8]);
        domain_input[32..].copy_from_slice(&intent_hash[..]);
        keccak256(&domain_input)
    }
}
