//! Fetcher trait definition.

use kona_preimage::{HintRouter, PreimageFetcher};

/// The Fetcher trait is used to define the interface for fetching data from the preimage oracle,
/// by [PreimageKey], and routing hints.
///
/// [PreimageKey]: kona_preimage::PreimageKey
pub trait Fetcher: PreimageFetcher + HintRouter {}

impl<T> Fetcher for T where T: PreimageFetcher + HintRouter {}
