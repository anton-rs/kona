//! Contains the cursor for the derivation pipeline.

use alloy_rpc_types_engine::ForkchoiceState;
use core::{
    fmt::{Display, Formatter},
    str::FromStr,
};
use maili_protocol::L2BlockInfo;

/// A cursor that keeps track of the heads of the L2 chain, relative to their [SafetyLabel]s.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct L2ChainHeads {
    /// The L2 unsafe tip.
    unsafe_: L2BlockInfo,
    /// The L2 cross-unsafe tip.
    cross_unsafe: L2BlockInfo,
    /// The L2 safe tip.
    safe: L2BlockInfo,
    /// The L2 cross-safe tip.
    cross_safe: L2BlockInfo,
    /// The finalized tip.
    finalized: L2BlockInfo,
}

impl L2ChainHeads {
    /// Constructs a new [L2ChainHeads] with the same head for all labels.
    pub const fn new_unified(head: L2BlockInfo) -> Self {
        Self { unsafe_: head, cross_unsafe: head, safe: head, cross_safe: head, finalized: head }
    }

    /// Constructs a new [L2ChainHeads] with the given heads for each label.
    pub const fn new(
        unsafe_: L2BlockInfo,
        cross_unsafe: L2BlockInfo,
        safe: L2BlockInfo,
        cross_safe: L2BlockInfo,
        finalized: L2BlockInfo,
    ) -> Self {
        Self { unsafe_, cross_unsafe, safe, cross_safe, finalized }
    }

    /// Advances the [SafetyLabel] head of the L2 chain.
    pub fn advance(&mut self, label: SafetyLabel, new_head: L2BlockInfo) {
        match label {
            SafetyLabel::Unsafe => self.advance_unsafe_head(new_head),
            SafetyLabel::CrossUnsafe => self.advance_cross_unsafe_head(new_head),
            SafetyLabel::Safe => self.advance_safe_head(new_head),
            SafetyLabel::CrossSafe => self.advance_cross_safe_head(new_head),
        }
    }

    /// Returns the L2 unsafe head.
    pub const fn unsafe_head(&self) -> &L2BlockInfo {
        &self.unsafe_
    }

    /// Advances the L2 unsafe head.
    pub fn advance_unsafe_head(&mut self, new_head: L2BlockInfo) {
        if new_head.block_info.parent_hash != self.unsafe_.block_info.hash {
            panic!("Attempted to advance the L2 unsafe head with a non-child block");
        }

        self.unsafe_ = new_head;
    }

    /// Returns the L2 cross-unsafe head.
    pub const fn cross_unsafe_head(&self) -> &L2BlockInfo {
        &self.cross_unsafe
    }

    /// Advances the L2 cross-unsafe head.
    pub fn advance_cross_unsafe_head(&mut self, new_head: L2BlockInfo) {
        if new_head.block_info.parent_hash != self.cross_unsafe.block_info.hash {
            panic!("Attempted to advance the L2 cross-unsafe head with a non-child block");
        }

        self.cross_unsafe = new_head;
    }

    /// Returns the L2 safe head.
    pub const fn safe_head(&self) -> &L2BlockInfo {
        &self.safe
    }

    /// Advances the L2 safe head.
    pub fn advance_safe_head(&mut self, new_head: L2BlockInfo) {
        if new_head.block_info.parent_hash != self.safe.block_info.hash {
            panic!("Attempted to advance the L2 safe head with a non-child block");
        }

        self.safe = new_head;
    }

    /// Returns the L2 cross-safe head.
    pub const fn cross_safe_head(&self) -> &L2BlockInfo {
        &self.cross_safe
    }

    /// Advances the L2 cross-safe head.
    pub fn advance_cross_safe_head(&mut self, new_head: L2BlockInfo) {
        if new_head.block_info.parent_hash != self.cross_safe.block_info.hash {
            panic!("Attempted to advance the L2 cross-safe head with a non-child block");
        }

        self.cross_safe = new_head;
    }

    /// Returns the finalized head.
    pub const fn finalized_head(&self) -> &L2BlockInfo {
        &self.finalized
    }

    /// Advances the finalized head.
    pub fn advance_finalized_head(&mut self, new_head: L2BlockInfo) {
        if new_head.block_info.parent_hash != self.finalized.block_info.hash {
            panic!("Attempted to advance the finalized head with a non-child block");
        }

        self.finalized = new_head;
    }
}

impl From<L2ChainHeads> for ForkchoiceState {
    fn from(value: L2ChainHeads) -> Self {
        ForkchoiceState {
            head_block_hash: value.unsafe_head().block_info.hash,
            safe_block_hash: value.safe_head().block_info.hash,
            // TODO: Finalized needs to be tracked.
            finalized_block_hash: value.cross_safe_head().block_info.hash,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SafetyLabel {
    /// The block is unsafe.
    Unsafe,
    /// The block is cross-unsafe.
    CrossUnsafe,
    /// The block is safe.
    Safe,
    /// The block is cross-safe.
    CrossSafe,
}

impl Display for SafetyLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            SafetyLabel::Unsafe => write!(f, "unsafe"),
            SafetyLabel::CrossUnsafe => write!(f, "cross-unsafe"),
            SafetyLabel::Safe => write!(f, "safe"),
            SafetyLabel::CrossSafe => write!(f, "cross-safe"),
        }
    }
}

impl FromStr for SafetyLabel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unsafe" => Ok(SafetyLabel::Unsafe),
            "cross-unsafe" => Ok(SafetyLabel::CrossUnsafe),
            "safe" => Ok(SafetyLabel::Safe),
            "cross-safe" => Ok(SafetyLabel::CrossSafe),
            _ => Err(()),
        }
    }
}
