//! Interop message primitives.
//!
//! <https://specs.optimism.io/interop/messaging.html#messaging>
//! <https://github.com/ethereum-optimism/optimism/blob/34d5f66ade24bd1f3ce4ce7c0a6cfc1a6540eca1/packages/contracts-bedrock/src/L2/CrossL2Inbox.sol>

use alloc::{vec, vec::Vec};
use alloy_primitives::{keccak256, Address, Bytes, Log, U256};
use alloy_sol_types::{sol, SolCall, SolType};

sol! {
    /// @notice The struct for a pointer to a message payload in a remote (or local) chain.
    #[derive(Default, Debug, PartialEq, Eq)]
    struct MessageIdentifierAbi {
        address origin;
        uint256 blockNumber;
        uint256 logIndex;
        uint256 timestamp;
        uint256 chainId;
    }

    /// @notice Emitted when a cross chain message is being executed.
    /// @param msgHash Hash of message payload being executed.
    /// @param id Encoded Identifier of the message.
    #[derive(Default, Debug, PartialEq, Eq)]
    event ExecutingMessage(bytes32 indexed msgHash, MessageIdentifierAbi id);

    /// @notice Executes a cross chain message on the destination chain.
    /// @param _id      Identifier of the message.
    /// @param _target  Target address to call.
    /// @param _message Message payload to call target with.
    function executeMessage(
        MessageIdentifierAbi calldata _id,
        address _target,
        bytes calldata _message
    ) external;
}

/// A [RawMessagePayload] is the raw payload of an initiating message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawMessagePayload(Bytes);

impl From<Log> for RawMessagePayload {
    fn from(log: Log) -> Self {
        let mut data = vec![0u8; log.topics().len() * 32 + log.data.data.len()];
        for (i, topic) in log.topics().iter().enumerate() {
            data[i * 32..(i + 1) * 32].copy_from_slice(topic.as_ref());
        }
        data[(log.topics().len() * 32)..].copy_from_slice(log.data.data.as_ref());
        data.into()
    }
}

impl From<Vec<u8>> for RawMessagePayload {
    fn from(data: Vec<u8>) -> Self {
        Self(Bytes::from(data))
    }
}

impl From<Bytes> for RawMessagePayload {
    fn from(bytes: Bytes) -> Self {
        Self(bytes)
    }
}

impl From<RawMessagePayload> for Bytes {
    fn from(payload: RawMessagePayload) -> Self {
        payload.0
    }
}

impl AsRef<[u8]> for RawMessagePayload {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// A [MessageIdentifier] uniquely represents a log that is emitted from a chain within
/// the broader dependency set. It is included in the calldata of a transaction sent to the
/// CrossL2Inbox contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageIdentifier {
    /// The account that sent the message.
    pub origin: Address,
    /// The block number that the message was sent in.
    pub block_number: u64,
    /// The log index of the message in the block (global).
    pub log_index: u64,
    /// The timestamp of the message.
    pub timestamp: u64,
    /// The chain ID of the chain that the message was sent on.
    pub chain_id: u64,
}

impl MessageIdentifier {
    /// Decode a [MessageIdentifier] from ABI-encoded data.
    pub fn abi_decode(data: &[u8], validate: bool) -> Result<Self, alloy_sol_types::Error> {
        MessageIdentifierAbi::abi_decode(data, validate).and_then(|abi| Ok(abi.into()))
    }
}

impl From<MessageIdentifierAbi> for MessageIdentifier {
    fn from(abi: MessageIdentifierAbi) -> Self {
        Self {
            origin: abi.origin,
            block_number: abi.blockNumber.to(),
            log_index: abi.logIndex.to(),
            timestamp: abi.timestamp.to(),
            chain_id: abi.chainId.to(),
        }
    }
}

impl From<MessageIdentifier> for MessageIdentifierAbi {
    fn from(id: MessageIdentifier) -> Self {
        Self {
            origin: id.origin,
            blockNumber: U256::from(id.block_number),
            logIndex: U256::from(id.log_index),
            timestamp: U256::from(id.timestamp),
            chainId: U256::from(id.chain_id),
        }
    }
}

impl From<executeMessageCall> for ExecutingMessage {
    fn from(call: executeMessageCall) -> Self {
        Self { id: call._id.into(), msgHash: keccak256(call._message.as_ref()) }
    }
}
