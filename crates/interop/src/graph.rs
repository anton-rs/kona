//! Dependency graph.

use crate::{constants::CROSS_L2_INBOX_ADDRESS, traits::InteropProvider, ExecutingMessage};
use alloc::vec::Vec;
use alloy_consensus::{Header, Sealed};

/// The dependency graph represents a set of blocks at a given timestamp and the interop
/// dependencies between them.
///
/// This structure is used to determine whether or not any interop messages are invalid within the
/// set of blocks within the graph. An "invalid message" is one that was relayed from one chain to
/// another, but the original [MessageIdentifier] is not present within the graph or from a
/// dependency referenced via the [InteropProvider].
///
/// [MessageIdentifier]: crate::MessageIdentifier
#[derive(Debug)]
pub struct DependencyGraph<P> {
    /// The L2 blocks within the graph.
    ///
    /// Represented as `(chain_id, block)`
    nodes: Vec<(u64, Sealed<Header>)>,
    /// The edges within the graph.
    ///
    /// These are derived from the transactions within the blocks. If [None], the graph has yet to
    /// be built.
    edges: Option<Vec<ExecutingMessage>>,
    /// The data provider for the graph.
    provider: P,
}

impl<P> DependencyGraph<P>
where
    P: InteropProvider,
{
    /// Creates a new dependency graph from a list of blocks (with chain IDs) and an
    /// [InteropProvider].
    pub fn new(blocks: &[(u64, Sealed<Header>)], provider: P) -> Self {
        Self { nodes: blocks.to_vec(), edges: Some(Vec::new()), provider }
    }

    /// Adds a block to the dependency graph. If [Self::edges] is [Some], the edges are discarded,
    /// and the graph must be rebuilt.
    pub fn add_block(&mut self, chain_id: u64, header: Sealed<Header>) {
        self.nodes.push((chain_id, header));
        self.edges = None;
    }

    /// Derives the edges from the blocks within the graph by scanning all transactions within the
    /// blocks and searching for [ExecutingMessage]s.
    async fn derive(&mut self) -> Result<(), ()> {
        let mut edges = self.edges.get_or_insert_default();

        // for (chain_id, block_header) in self.nodes.iter() {
        //     let receipts = self.provider.block_receipts(*chain_id, block_header.hash_slow()).await;
        //
        //     for receipt in receipts {
        //         for log in receipt.logs() {
        //             if log.address == CROSS_L2_INBOX_ADDRESS {
        //                 let message = ExecutingMessage::from(log.clone());
        //                 self.edges.as_mut().unwrap().push(message);
        //             }
        //         }
        //     }
        // }

        Ok(())
    }
}
