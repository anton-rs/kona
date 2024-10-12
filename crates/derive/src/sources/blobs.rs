//! Blob Data Source

use crate::{
    errors::{BlobDecodingError, BlobProviderError, PipelineError, PipelineResult},
    traits::{AsyncIterator, BlobProvider},
};
use alloc::{boxed::Box, format, string::ToString, vec, vec::Vec};
use alloy_consensus::{Transaction, TxEip4844Variant, TxEnvelope, TxType};
use alloy_eips::{
    eip1898::NumHash,
    eip4844::{Blob, BYTES_PER_BLOB, VERSIONED_HASH_VERSION_KZG},
};
use alloy_primitives::{Address, Bytes, TxKind};
use async_trait::async_trait;
use kona_providers::ChainProvider;
use op_alloy_protocol::BlockInfo;
use tracing::warn;

/// The blob encoding version
pub(crate) const BLOB_ENCODING_VERSION: u8 = 0;

/// Maximum blob data size
pub(crate) const BLOB_MAX_DATA_SIZE: usize = (4 * 31 + 3) * 1024 - 4; // 130044

/// Blob Encoding/Decoding Rounds
pub(crate) const BLOB_ENCODING_ROUNDS: usize = 1024;

/// The Blob Data
#[derive(Default, Clone, Debug)]
pub struct BlobData {
    /// The blob data
    pub(crate) data: Option<Bytes>,
    /// The calldata
    pub(crate) calldata: Option<Bytes>,
}

impl BlobData {
    /// Decodes the blob into raw byte data.
    /// Returns a [BlobDecodingError] if the blob is invalid.
    pub(crate) fn decode(&self) -> Result<Bytes, BlobDecodingError> {
        let data = self.data.as_ref().ok_or(BlobDecodingError::MissingData)?;

        // Validate the blob encoding version
        if data[VERSIONED_HASH_VERSION_KZG as usize] != BLOB_ENCODING_VERSION {
            return Err(BlobDecodingError::InvalidEncodingVersion);
        }

        // Decode the 3 byte big endian length value into a 4 byte integer
        let length = u32::from_be_bytes([0, data[2], data[3], data[4]]) as usize;

        // Validate the length
        if length > BLOB_MAX_DATA_SIZE {
            return Err(BlobDecodingError::InvalidLength);
        }

        // Round 0 copies the remaining 27 bytes of the first field element
        let mut output = vec![0u8; BLOB_MAX_DATA_SIZE];
        output[0..27].copy_from_slice(&data[5..32]);

        // Process the remaining 3 field elements to complete round 0
        let mut output_pos = 28;
        let mut input_pos = 32;
        let mut encoded_byte = [0u8; 4];
        encoded_byte[0] = data[0];

        for b in encoded_byte.iter_mut().skip(1) {
            let (enc, opos, ipos) =
                self.decode_field_element(output_pos, input_pos, &mut output)?;
            *b = enc;
            output_pos = opos;
            input_pos = ipos;
        }

        // Reassemble the 4 by 6 bit encoded chunks into 3 bytes of output
        output_pos = self.reassemble_bytes(output_pos, &encoded_byte, &mut output);

        // In each remaining round, decode 4 field elements (128 bytes) of the
        // input into 127 bytes of output
        for _ in 1..BLOB_ENCODING_ROUNDS {
            // Break early if the output position is greater than the length
            if output_pos >= length {
                break;
            }

            for d in &mut encoded_byte {
                let (enc, opos, ipos) =
                    self.decode_field_element(output_pos, input_pos, &mut output)?;
                *d = enc;
                output_pos = opos;
                input_pos = ipos;
            }
            output_pos = self.reassemble_bytes(output_pos, &encoded_byte, &mut output);
        }

        // Validate the remaining bytes
        for o in output.iter().skip(length) {
            if *o != 0u8 {
                return Err(BlobDecodingError::InvalidFieldElement);
            }
        }

        // Validate the remaining bytes
        output.truncate(length);
        for i in input_pos..BYTES_PER_BLOB {
            if data[i] != 0 {
                return Err(BlobDecodingError::InvalidFieldElement);
            }
        }

        Ok(Bytes::from(output))
    }

    /// Decodes the next input field element by writing its lower 31 bytes into its
    /// appropriate place in the output and checking the high order byte is valid.
    /// Returns a [BlobDecodingError] if a field element is seen with either of its
    /// two high order bits set.
    pub(crate) fn decode_field_element(
        &self,
        output_pos: usize,
        input_pos: usize,
        output: &mut [u8],
    ) -> Result<(u8, usize, usize), BlobDecodingError> {
        let Some(data) = self.data.as_ref() else {
            return Err(BlobDecodingError::MissingData);
        };

        // two highest order bits of the first byte of each field element should always be 0
        if data[input_pos] & 0b1100_0000 != 0 {
            return Err(BlobDecodingError::InvalidFieldElement);
        }
        output[output_pos..output_pos + 31].copy_from_slice(&data[input_pos + 1..input_pos + 32]);
        Ok((data[input_pos], output_pos + 32, input_pos + 32))
    }

    /// Reassemble 4 by 6 bit encoded chunks into 3 bytes of output and place them in their
    /// appropriate output positions.
    pub(crate) fn reassemble_bytes(
        &self,
        mut output_pos: usize,
        encoded_byte: &[u8],
        output: &mut [u8],
    ) -> usize {
        output_pos -= 1;
        let x = (encoded_byte[0] & 0b0011_1111) | ((encoded_byte[1] & 0b0011_0000) << 2);
        let y = (encoded_byte[1] & 0b0000_1111) | ((encoded_byte[3] & 0b0000_1111) << 4);
        let z = (encoded_byte[2] & 0b0011_1111) | ((encoded_byte[3] & 0b0011_0000) << 2);
        output[output_pos - 32] = z;
        output[output_pos - (32 * 2)] = y;
        output[output_pos - (32 * 3)] = x;
        output_pos
    }

    /// Fills in the pointers to the fetched blob bodies.
    /// There should be exactly one placeholder blobOrCalldata
    /// element for each blob, otherwise an error is returned.
    pub(crate) fn fill(
        &mut self,
        blobs: &[Box<Blob>],
        index: usize,
    ) -> Result<(), BlobDecodingError> {
        // Do not fill if there is no calldata to fill
        if self.calldata.as_ref().map_or(false, |data| data.is_empty()) {
            return Ok(());
        }

        if index >= blobs.len() {
            return Err(BlobDecodingError::InvalidLength);
        }

        if blobs[index].is_empty() {
            return Err(BlobDecodingError::MissingData);
        }

        self.data = Some(Bytes::from(*blobs[index]));
        Ok(())
    }
}

/// A data iterator that reads from a blob.
#[derive(Debug, Clone)]
pub struct BlobSource<F, B>
where
    F: ChainProvider + Send,
    B: BlobProvider + Send,
{
    /// Chain provider.
    pub chain_provider: F,
    /// Fetches blobs.
    pub blob_fetcher: B,
    /// The address of the batcher contract.
    pub batcher_address: Address,
    /// Block Ref
    pub block_ref: BlockInfo,
    /// The L1 Signer.
    pub signer: Address,
    /// Data.
    pub data: Vec<BlobData>,
    /// Whether the source is open.
    pub open: bool,
}

impl<F, B> BlobSource<F, B>
where
    F: ChainProvider + Send,
    B: BlobProvider + Send,
{
    /// Creates a new blob source.
    pub const fn new(
        chain_provider: F,
        blob_fetcher: B,
        batcher_address: Address,
        block_ref: BlockInfo,
        signer: Address,
    ) -> Self {
        Self {
            chain_provider,
            blob_fetcher,
            batcher_address,
            block_ref,
            signer,
            data: Vec::new(),
            open: false,
        }
    }

    fn extract_blob_data(&self, txs: Vec<TxEnvelope>) -> (Vec<BlobData>, Vec<NumHash>) {
        let mut number: u64 = 0;
        let mut data = Vec::new();
        let mut hashes = Vec::new();
        for tx in txs {
            let (tx_kind, calldata, blob_hashes) = match &tx {
                TxEnvelope::Legacy(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip2930(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip1559(tx) => (tx.tx().to(), tx.tx().input.clone(), None),
                TxEnvelope::Eip4844(blob_tx_wrapper) => match blob_tx_wrapper.tx() {
                    TxEip4844Variant::TxEip4844(tx) => {
                        (tx.to(), tx.input.clone(), Some(tx.blob_versioned_hashes.clone()))
                    }
                    TxEip4844Variant::TxEip4844WithSidecar(tx) => {
                        let tx = tx.tx();
                        (tx.to(), tx.input.clone(), Some(tx.blob_versioned_hashes.clone()))
                    }
                },
                _ => continue,
            };
            let TxKind::Call(to) = tx_kind else { continue };

            if to != self.batcher_address {
                number += blob_hashes.map_or(0, |h| h.len() as u64);
                continue;
            }
            if tx.recover_signer().unwrap_or_default() != self.signer {
                number += blob_hashes.map_or(0, |h| h.len() as u64);
                continue;
            }
            if tx.tx_type() != TxType::Eip4844 {
                let blob_data = BlobData { data: None, calldata: Some(calldata.to_vec().into()) };
                data.push(blob_data);
                continue;
            }
            if !calldata.is_empty() {
                let hash = match &tx {
                    TxEnvelope::Legacy(tx) => Some(tx.hash()),
                    TxEnvelope::Eip2930(tx) => Some(tx.hash()),
                    TxEnvelope::Eip1559(tx) => Some(tx.hash()),
                    TxEnvelope::Eip4844(blob_tx_wrapper) => Some(blob_tx_wrapper.hash()),
                    _ => None,
                };
                warn!(target: "blob-source", "Blob tx has calldata, which will be ignored: {hash:?}");
            }
            let blob_hashes = if let Some(b) = blob_hashes {
                b
            } else {
                continue;
            };
            for blob in blob_hashes {
                let indexed = NumHash { hash: blob, number };
                hashes.push(indexed);
                data.push(BlobData::default());
                number += 1;
            }
        }
        (data, hashes)
    }

    /// Loads blob data into the source if it is not open.
    async fn load_blobs(&mut self) -> Result<(), BlobProviderError> {
        if self.open {
            return Ok(());
        }

        let info = self
            .chain_provider
            .block_info_and_transactions_by_hash(self.block_ref.hash)
            .await
            .map_err(|e| BlobProviderError::Backend(e.to_string()))?;

        let (mut data, blob_hashes) = self.extract_blob_data(info.1);

        // If there are no hashes, set the calldata and return.
        if blob_hashes.is_empty() {
            self.open = true;
            self.data = data;
            return Ok(());
        }

        let blobs =
            self.blob_fetcher.get_blobs(&self.block_ref, &blob_hashes).await.map_err(|e| {
                warn!(target: "blob-source", "Failed to fetch blobs: {e}");
                BlobProviderError::Backend(e.to_string())
            })?;

        // Fill the blob pointers.
        let mut blob_index = 0;
        for blob in data.iter_mut() {
            match blob.fill(&blobs, blob_index) {
                Ok(_) => {
                    blob_index += 1;
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        self.open = true;
        self.data = data;
        Ok(())
    }

    /// Extracts the next data from the source.
    fn next_data(&mut self) -> Result<BlobData, PipelineResult<Bytes>> {
        if self.data.is_empty() {
            return Err(Err(PipelineError::Eof.temp()));
        }

        Ok(self.data.remove(0))
    }
}

#[async_trait]
impl<F, B> AsyncIterator for BlobSource<F, B>
where
    F: ChainProvider + Send,
    B: BlobProvider + Send,
{
    type Item = Bytes;

    async fn next(&mut self) -> PipelineResult<Self::Item> {
        if self.load_blobs().await.is_err() {
            return Err(PipelineError::Provider(format!(
                "Failed to load blobs from stream: {}",
                self.block_ref.hash
            ))
            .temp());
        }

        let next_data = match self.next_data() {
            Ok(d) => d,
            Err(e) => return e,
        };
        if let Some(c) = next_data.calldata {
            return Ok(c);
        }

        // Decode the blob data to raw bytes.
        // Otherwise, ignore blob and recurse next.
        match next_data.decode() {
            Ok(d) => Ok(d),
            Err(_) => {
                warn!(target: "blob-source", "Failed to decode blob data, skipping");
                self.next().await
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{errors::PipelineErrorKind, traits::test_utils::TestBlobProvider};
    use alloy_rlp::Decodable;
    use kona_providers::test_utils::TestChainProvider;

    #[test]
    fn test_reassemble_bytes() {
        let blob_data = BlobData::default();
        let mut output = vec![0u8; 128];
        let encoded_byte = [0x00, 0x00, 0x00, 0x00];
        let output_pos = blob_data.reassemble_bytes(127, &encoded_byte, &mut output);
        assert_eq!(output_pos, 126);
        assert_eq!(output, vec![0u8; 128]);
    }

    #[test]
    fn test_cannot_fill_empty_calldata() {
        let mut blob_data = BlobData { calldata: Some(Bytes::new()), ..Default::default() };
        let blobs = vec![Box::new(Blob::with_last_byte(1u8))];
        assert_eq!(blob_data.fill(&blobs, 0), Ok(()));
    }

    #[test]
    fn test_fill_oob_index() {
        let mut blob_data = BlobData::default();
        let blobs = vec![Box::new(Blob::with_last_byte(1u8))];
        assert_eq!(blob_data.fill(&blobs, 1), Err(BlobDecodingError::InvalidLength));
    }

    #[test]
    #[ignore]
    fn test_fill_empty_blob() {
        let mut blob_data = BlobData::default();
        let blobs = vec![Box::new(Blob::ZERO)];
        assert_eq!(blob_data.fill(&blobs, 0), Err(BlobDecodingError::MissingData));
    }

    #[test]
    fn test_fill_blob() {
        let mut blob_data = BlobData::default();
        let blobs = vec![Box::new(Blob::with_last_byte(1u8))];
        assert_eq!(blob_data.fill(&blobs, 0), Ok(()));
        let expected = Bytes::from([&[0u8; 131071][..], &[1u8]].concat());
        assert_eq!(blob_data.data, Some(expected));
    }

    #[test]
    fn test_blob_data_decode_missing_data() {
        let blob_data = BlobData::default();
        assert_eq!(blob_data.decode(), Err(BlobDecodingError::MissingData));
    }

    #[test]
    fn test_blob_data_decode_invalid_encoding_version() {
        let blob_data = BlobData { data: Some(Bytes::from(vec![1u8; 32])), ..Default::default() };
        assert_eq!(blob_data.decode(), Err(BlobDecodingError::InvalidEncodingVersion));
    }

    #[test]
    fn test_blob_data_decode_invalid_length() {
        let mut data = vec![0u8; 32];
        data[VERSIONED_HASH_VERSION_KZG as usize] = BLOB_ENCODING_VERSION;
        data[2] = 0xFF;
        data[3] = 0xFF;
        data[4] = 0xFF;
        let blob_data = BlobData { data: Some(Bytes::from(data)), ..Default::default() };
        assert_eq!(blob_data.decode(), Err(BlobDecodingError::InvalidLength));
    }

    pub(crate) fn default_test_blob_source() -> BlobSource<TestChainProvider, TestBlobProvider> {
        let chain_provider = TestChainProvider::default();
        let blob_fetcher = TestBlobProvider::default();
        let batcher_address = Address::default();
        let block_ref = BlockInfo::default();
        let signer = Address::default();
        BlobSource::new(chain_provider, blob_fetcher, batcher_address, block_ref, signer)
    }

    pub(crate) fn valid_blob_txs() -> Vec<TxEnvelope> {
        // https://sepolia.etherscan.io/getRawTx?tx=0x9a22ccb0029bc8b0ddd073be1a1d923b7ae2b2ea52100bae0db4424f9107e9c0
        let raw_tx = alloy_primitives::hex::decode("0x03f9011d83aa36a7820fa28477359400852e90edd0008252089411e9ca82a3a762b4b5bd264d4173a242e7a770648080c08504a817c800f8a5a0012ec3d6f66766bedb002a190126b3549fce0047de0d4c25cffce0dc1c57921aa00152d8e24762ff22b1cfd9f8c0683786a7ca63ba49973818b3d1e9512cd2cec4a0013b98c6c83e066d5b14af2b85199e3d4fc7d1e778dd53130d180f5077e2d1c7a001148b495d6e859114e670ca54fb6e2657f0cbae5b08063605093a4b3dc9f8f1a0011ac212f13c5dff2b2c6b600a79635103d6f580a4221079951181b25c7e654901a0c8de4cced43169f9aa3d36506363b2d2c44f6c49fc1fd91ea114c86f3757077ea01e11fdd0d1934eda0492606ee0bb80a7bf8f35cc5f86ec60fe5031ba48bfd544").unwrap();
        let eip4844 = TxEnvelope::decode(&mut raw_tx.as_slice()).unwrap();
        vec![eip4844]
    }

    #[tokio::test]
    async fn test_load_blobs_open() {
        let mut source = default_test_blob_source();
        source.open = true;
        assert!(source.load_blobs().await.is_ok());
    }

    #[tokio::test]
    async fn test_load_blobs_chain_provider_err() {
        let mut source = default_test_blob_source();
        assert!(matches!(source.load_blobs().await, Err(BlobProviderError::Backend(_))));
    }

    #[tokio::test]
    async fn test_load_blobs_chain_provider_empty_txs() {
        let mut source = default_test_blob_source();
        let block_info = BlockInfo::default();
        source.chain_provider.insert_block_with_transactions(0, block_info, Vec::new());
        assert!(!source.open); // Source is not open by default.
        assert!(source.load_blobs().await.is_ok());
        assert!(source.data.is_empty());
        assert!(source.open);
    }

    #[tokio::test]
    async fn test_load_blobs_chain_provider_4844_txs_blob_fetch_error() {
        let mut source = default_test_blob_source();
        let block_info = BlockInfo::default();
        source.signer = alloy_primitives::address!("A83C816D4f9b2783761a22BA6FADB0eB0606D7B2");
        source.batcher_address =
            alloy_primitives::address!("11E9CA82A3a762b4B5bd264d4173a242e7a77064");
        let txs = valid_blob_txs();
        source.blob_fetcher.should_error = true;
        source.chain_provider.insert_block_with_transactions(1, block_info, txs);
        assert!(matches!(source.load_blobs().await, Err(BlobProviderError::Backend(_))));
    }

    #[tokio::test]
    async fn test_load_blobs_chain_provider_4844_txs_succeeds() {
        use alloy_consensus::Blob;

        let mut source = default_test_blob_source();
        let block_info = BlockInfo::default();
        source.signer = alloy_primitives::address!("A83C816D4f9b2783761a22BA6FADB0eB0606D7B2");
        source.batcher_address =
            alloy_primitives::address!("11E9CA82A3a762b4B5bd264d4173a242e7a77064");
        let txs = valid_blob_txs();
        source.chain_provider.insert_block_with_transactions(1, block_info, txs);
        let hashes = [
            alloy_primitives::b256!(
                "012ec3d6f66766bedb002a190126b3549fce0047de0d4c25cffce0dc1c57921a"
            ),
            alloy_primitives::b256!(
                "0152d8e24762ff22b1cfd9f8c0683786a7ca63ba49973818b3d1e9512cd2cec4"
            ),
            alloy_primitives::b256!(
                "013b98c6c83e066d5b14af2b85199e3d4fc7d1e778dd53130d180f5077e2d1c7"
            ),
            alloy_primitives::b256!(
                "01148b495d6e859114e670ca54fb6e2657f0cbae5b08063605093a4b3dc9f8f1"
            ),
            alloy_primitives::b256!(
                "011ac212f13c5dff2b2c6b600a79635103d6f580a4221079951181b25c7e6549"
            ),
        ];
        for hash in hashes {
            source.blob_fetcher.insert_blob(hash, Blob::with_last_byte(1u8));
        }
        source.load_blobs().await.unwrap();
        assert!(source.open);
        assert!(!source.data.is_empty());
    }

    #[tokio::test]
    async fn test_open_empty_data_eof() {
        let mut source = default_test_blob_source();
        source.open = true;

        let err = source.next().await.unwrap_err();
        assert!(matches!(err, PipelineErrorKind::Temporary(PipelineError::Eof)));
    }

    #[tokio::test]
    async fn test_open_calldata() {
        let mut source = default_test_blob_source();
        source.open = true;
        source.data.push(BlobData { data: None, calldata: Some(Bytes::default()) });

        let data = source.next().await.unwrap();
        assert_eq!(data, Bytes::default());
    }

    #[tokio::test]
    async fn test_open_blob_data_decode_missing_data() {
        let mut source = default_test_blob_source();
        source.open = true;
        source.data.push(BlobData { data: Some(Bytes::from(&[1; 32])), calldata: None });

        let err = source.next().await.unwrap_err();
        assert!(matches!(err, PipelineErrorKind::Temporary(PipelineError::Eof)));
    }

    #[tokio::test]
    async fn test_blob_source_pipeline_error() {
        let mut source = default_test_blob_source();

        let err = source.next().await.unwrap_err();
        assert!(matches!(err, PipelineErrorKind::Temporary(PipelineError::Provider(_))));
    }
}
