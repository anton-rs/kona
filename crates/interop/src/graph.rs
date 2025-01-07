//! Interop dependency graph implementation.

use crate::{
    errors::{DependencyGraphError, DependencyGraphResult},
    message::{extract_executing_messages, EnrichedExecutingMessage},
    traits::InteropProvider,
    RawMessagePayload,
};
use alloc::vec::Vec;
use alloy_consensus::{Header, Sealed};
use alloy_primitives::{hex, keccak256};
use tracing::{error, info, warn};

/// The dependency graph represents a set of blocks at a given timestamp and the interop
/// dependencies between them.
///
/// This structure is used to determine whether or not any interop messages are invalid within the
/// set of blocks within the graph. An "invalid message" is one that was relayed from one chain to
/// another, but the original [MessageIdentifier] is not present within the graph or from a
/// dependency referenced via the [InteropProvider] (or otherwise is invalid, such as being older
/// than the message expiry window).
///
/// [MessageIdentifier]: crate::MessageIdentifier
#[derive(Debug)]
pub struct DependencyGraph<P> {
    /// The horizon timestamp is the timestamp of all blocks containing [ExecutingMessage]s within
    /// the graph.
    horizon_timestamp: u64,
    /// The edges within the graph.
    ///
    /// These are derived from the transactions within the blocks.
    messages: Vec<EnrichedExecutingMessage>,
    /// The data provider for the graph. Required for fetching headers, receipts and remote messages
    /// within history during resolution.
    provider: P,
}

impl<P> DependencyGraph<P>
where
    P: InteropProvider,
{
    /// Derives the edges from the blocks within the graph by scanning all receipts within the
    /// blocks and searching for [ExecutingMessage]s.
    ///
    /// [ExecutingMessage]: crate::ExecutingMessage
    pub async fn derive(
        blocks: &[(u64, Sealed<Header>)],
        provider: P,
    ) -> DependencyGraphResult<Self> {
        info!(
            target: "dependency-graph",
            "Deriving dependency graph from {} blocks.",
            blocks.len()
        );

        // Get the first timestamp to compare against
        let horizon_timestamp = blocks
            .first()
            .map(|(_, header)| header.inner().timestamp)
            .ok_or(DependencyGraphError::EmptyDependencySet)?;

        // Verify all blocks have the same timestamp
        if !blocks.iter().all(|(_, header)| header.inner().timestamp == horizon_timestamp) {
            error!(
                target: "dependency-graph",
                "Mismatched timestamps in blocks. Must have a unified horizon timestamp."
            );
            return Err(DependencyGraphError::MismatchedTimestamps);
        }

        let mut messages = Vec::new();
        for (chain_id, header) in blocks.iter() {
            let receipts = provider.receipts_by_hash(*chain_id, header.hash()).await?;
            let executing_messages = extract_executing_messages(receipts.as_slice());

            messages.extend(
                executing_messages
                    .into_iter()
                    .map(|message| EnrichedExecutingMessage::new(message, *chain_id)),
            );
        }

        info!(
            target: "dependency-graph",
            "Derived {} executing messages from {} blocks.",
            messages.len(),
            blocks.len()
        );
        Ok(Self { horizon_timestamp, messages, provider })
    }

    /// Checks the validity of all messages within the graph.
    pub async fn resolve(mut self) -> DependencyGraphResult<()> {
        info!(
            target: "dependency-graph",
            "Checking the dependency graph for invalid messages."
        );

        // Reduce the graph to remove all valid messages.
        self.reduce().await?;

        // Check if the graph is now empty. If not, there are invalid messages.
        if !self.messages.is_empty() {
            // Collect the chain IDs for all blocks containing invalid messages.
            let mut bad_block_chain_ids =
                self.messages.into_iter().map(|e| e.executing_chain_id).collect::<Vec<_>>();
            bad_block_chain_ids.dedup_by(|a, b| a == b);

            warn!(
                target: "dependency-graph",
                "Failed to reduce the dependency graph entirely. Invalid messages found in chains {}",
                bad_block_chain_ids
                    .iter()
                    .map(|id| alloc::format!("{}", id))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            // Return an error with the chain IDs of the blocks containing invalid messages.
            return Err(DependencyGraphError::InvalidMessages(bad_block_chain_ids));
        }

        Ok(())
    }

    /// Attempts to remove as many edges from the graph as possible by resolving the dependencies
    /// of each message. If a message cannot be resolved, it is considered invalid. After this
    /// function is called, any outstanding edges mark invalid messages, and therefore invalid
    /// nodes.
    async fn reduce(&mut self) -> DependencyGraphResult<()> {
        // Create a new vector to store invalid edges
        let mut invalid_messages = Vec::with_capacity(self.messages.len());

        // Prune all valid edges.
        for message in core::mem::take(&mut self.messages) {
            if let Err(e) = self.check_single_dependency(&message).await {
                warn!(
                    target: "dependency-graph",
                    "Invalid ExecutingMessage found - relayed on chain {} with message hash {}.",
                    message.executing_chain_id,
                    hex::encode(message.inner.msgHash)
                );
                warn!("Invalid message error: {}", e);
                invalid_messages.push(message);
            }
        }

        info!(
            target: "dependency-graph",
            "Successfully reduced the dependency graph. {} invalid messages found.",
            invalid_messages.len()
        );

        // Replace the old edges with the filtered list
        self.messages = invalid_messages;

        Ok(())
    }

    /// Checks the dependency of a single [ExecutingMessageWithChain]. If the message's dependencies are
    /// unavailable, the message is considered invalid and an [Err] is returned.
    async fn check_single_dependency(
        &self,
        message: &EnrichedExecutingMessage,
    ) -> DependencyGraphResult<()> {
        // ChainID Invariant: The chain id of the initiating message MUST be in the dependency set
        // This is enforced implicitly by the graph constructor and the provider.

        // Timestamp invariant: The timestamp at the time of inclusion of the initiating message
        // MUST be less than or equal to the timestamp of the executing message as well as greater
        // than or equal to the Interop Start Timestamp.
        if message.inner.id.timestamp.saturating_to::<u64>() > self.horizon_timestamp {
            todo!("Interop fork activation check - need rollup config.");
            return Err(DependencyGraphError::MessageInFuture(
                self.horizon_timestamp,
                message.inner.id.timestamp.saturating_to(),
            ));
        }

        // Fetch the header & receipts for the message's claimed origin block on the remote chain.
        let remote_header = self
            .provider
            .header_by_number(
                message.inner.id.chainId.saturating_to(),
                message.inner.id.blockNumber.saturating_to(),
            )
            .await?;
        let remote_receipts = self
            .provider
            .receipts_by_number(
                message.inner.id.chainId.saturating_to(),
                message.inner.id.blockNumber.saturating_to(),
            )
            .await?;

        // Find the log that matches the message's claimed log index. Note that the
        // log index is global to the block, so we chain the full block's logs together
        // to find it.
        let remote_log = remote_receipts
            .iter()
            .flat_map(|receipt| receipt.logs())
            .nth(message.inner.id.logIndex.saturating_to())
            .ok_or(DependencyGraphError::RemoteMessageNotFound(
                message.inner.id.chainId.to(),
                message.inner.msgHash,
            ))?;

        // Validate the message's origin is correct.
        if remote_log.address != message.inner.id.origin {
            return Err(DependencyGraphError::InvalidMessageOrigin(
                message.inner.id.origin,
                remote_log.address,
            ));
        }

        // Validate that the message hash is correct.
        let remote_message = RawMessagePayload::from(remote_log);
        let remote_message_hash = keccak256(remote_message.as_ref());
        if remote_message_hash != message.inner.msgHash {
            return Err(DependencyGraphError::InvalidMessageHash(
                message.inner.msgHash,
                remote_message_hash,
            ));
        }

        // Validate that the timestamp of the block header containing the log is correct.
        if remote_header.timestamp != message.inner.id.timestamp.saturating_to::<u64>() {
            return Err(DependencyGraphError::InvalidMessageTimestamp(
                message.inner.id.timestamp.saturating_to::<u64>(),
                remote_header.timestamp,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::DependencyGraph;
    use crate::{
        constants::CROSS_L2_INBOX_ADDRESS, message::MessageIdentifier,
        test_utils::MockInteropProvider, ExecutingMessage,
    };
    use alloy_consensus::{Header, Receipt, ReceiptWithBloom};
    use alloy_primitives::{keccak256, map::HashMap, Address, Log, LogData, U256};
    use alloy_sol_types::{sol, SolEvent, SolValue};
    use op_alloy_consensus::OpReceiptEnvelope;

    sol! {
        event MockMessage(bytes32 indexed _id);
    }

    #[tokio::test]
    async fn test_derive_and_reduce_simple_graph() {
        // Chain A
        let header_a = Header { timestamp: 2, ..Default::default() };
        let header_a_hash = header_a.hash_slow();
        let sealed_header_a = header_a.seal(header_a_hash);
        let receipts_a = vec![OpReceiptEnvelope::Eip1559(ReceiptWithBloom {
            receipt: Receipt {
                logs: vec![Log {
                    address: Address::ZERO,
                    data: LogData::new(vec![MockMessage::SIGNATURE_HASH], Default::default())
                        .unwrap(),
                }],
                ..Default::default()
            },
            ..Default::default()
        })];

        // Chain B
        let header_b = Header { timestamp: 2, ..Default::default() };
        let header_b_hash = header_b.hash_slow();
        let sealed_header_b = header_b.seal(header_b_hash);
        let receipts_b = vec![OpReceiptEnvelope::Eip1559(ReceiptWithBloom {
            receipt: Receipt {
                logs: vec![Log {
                    address: CROSS_L2_INBOX_ADDRESS,
                    data: LogData::new(
                        vec![
                            ExecutingMessage::SIGNATURE_HASH,
                            keccak256(MockMessage::SIGNATURE_HASH),
                        ],
                        MessageIdentifier {
                            origin: Address::ZERO,
                            blockNumber: U256::ZERO,
                            logIndex: U256::from(2),
                            timestamp: U256::from(2),
                            chainId: U256::from(1),
                        }
                        .abi_encode()
                        .into(),
                    )
                    .unwrap(),
                }],
                ..Default::default()
            },
            ..Default::default()
        })];

        let mut headers = HashMap::new();
        headers.insert(1, HashMap::from([(0, sealed_header_a.clone())]));
        headers.insert(2, HashMap::from([(0, sealed_header_b.clone())]));

        let mut receipts = HashMap::new();
        receipts.insert(1, HashMap::from([(0, receipts_a)]));
        receipts.insert(2, HashMap::from([(0, receipts_b)]));

        let provider = MockInteropProvider::new(headers, receipts);
        let graph =
            DependencyGraph::derive(&[(1, sealed_header_a), (2, sealed_header_b)], provider)
                .await
                .unwrap();
        graph.resolve().await.unwrap();
    }
}
