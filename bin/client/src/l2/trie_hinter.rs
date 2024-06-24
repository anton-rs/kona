//! Contains the hinter for the [TrieDB].
//!
//! [TrieDB]: kona_mpt::TrieDB

use crate::{HintType, HINT_WRITER};
use alloy_primitives::{Address, B256};
use kona_mpt::TrieDBHinter;
use anyhow::Result;
use kona_preimage::HintWriterClient;

/// The [TrieDBHinter] implementation for the block executor's [TrieDB].
///
/// [TrieDB]: kona_mpt::TrieDB
#[derive(Debug)]
pub struct TrieDBHintWriter;
