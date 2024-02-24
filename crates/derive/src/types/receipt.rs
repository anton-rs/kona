//! This module contains the receipt types used within the derivation pipeline.

use core::cmp::Ordering;

use crate::types::transaction::TxType;
use alloc::vec::Vec;
use alloy_primitives::{Bloom, Log};
use alloy_rlp::{length_of_length, Buf, BufMut, BytesMut, Decodable, Encodable};

/// Receipt containing result of transaction execution.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Receipt {
    /// The transaction type of the receipt.
    pub tx_type: TxType,
    /// If transaction is executed successfully.
    ///
    /// This is the `statusCode`
    pub success: bool,
    /// Gas used
    pub cumulative_gas_used: u64,
    /// Log send from contracts.
    pub logs: Vec<Log>,
    /// Deposit nonce for Optimism deposit transactions
    pub deposit_nonce: Option<u64>,
    /// Deposit receipt version for Optimism deposit transactions
    ///
    ///
    /// The deposit receipt version was introduced in Canyon to indicate an update to how
    /// receipt hashes should be computed when set. The state transition process
    /// ensures this is only set for post-Canyon deposit transactions.
    pub deposit_receipt_version: Option<u64>,
}

impl Receipt {
    /// Calculates [`Log`]'s bloom filter. this is slow operation and [ReceiptWithBloom] can
    /// be used to cache this value.
    pub fn bloom_slow(&self) -> Bloom {
        self.logs.iter().collect()
    }

    /// Calculates the bloom filter for the receipt and returns the [ReceiptWithBloom] container
    /// type.
    pub fn with_bloom(self) -> ReceiptWithBloom {
        self.into()
    }
}

/// [`Receipt`] with calculated bloom filter.
///
/// This convenience type allows us to lazily calculate the bloom filter for a
/// receipt, similar to `Sealed`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ReceiptWithBloom {
    /// The receipt.
    pub receipt: Receipt,
    /// The bloom filter.
    pub bloom: Bloom,
}

impl From<Receipt> for ReceiptWithBloom {
    fn from(receipt: Receipt) -> Self {
        let bloom = receipt.bloom_slow();
        ReceiptWithBloom { receipt, bloom }
    }
}

impl ReceiptWithBloom {
    /// Create new [ReceiptWithBloom]
    pub const fn new(receipt: Receipt, bloom: Bloom) -> Self {
        Self { receipt, bloom }
    }

    /// Consume the structure, returning only the receipt
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_receipt(self) -> Receipt {
        self.receipt
    }

    /// Consume the structure, returning the receipt and the bloom filter
    #[allow(clippy::missing_const_for_fn)] // false positive
    pub fn into_components(self) -> (Receipt, Bloom) {
        (self.receipt, self.bloom)
    }

    fn payload_len(&self) -> usize {
        let mut payload_len = self.receipt.success.length()
            + self.receipt.cumulative_gas_used.length()
            + self.bloom.length()
            + self.receipt.logs.len();
        if self.receipt.tx_type == TxType::Deposit {
            if let Some(deposit_nonce) = self.receipt.deposit_nonce {
                payload_len += deposit_nonce.length();
            }
            if let Some(deposit_receipt_version) = self.receipt.deposit_receipt_version {
                payload_len += deposit_receipt_version.length();
            }
        }
        payload_len
    }

    /// Returns the rlp header for the receipt payload.
    fn receipt_rlp_header(&self) -> alloy_rlp::Header {
        alloy_rlp::Header {
            list: true,
            payload_length: self.payload_len(),
        }
    }

    /// Encodes the receipt data.
    fn encode_fields(&self, out: &mut dyn BufMut) {
        self.receipt_rlp_header().encode(out);
        self.receipt.success.encode(out);
        self.receipt.cumulative_gas_used.encode(out);
        self.bloom.encode(out);
        self.receipt.logs.encode(out);

        if self.receipt.tx_type == TxType::Deposit {
            if let Some(deposit_nonce) = self.receipt.deposit_nonce {
                deposit_nonce.encode(out)
            }
            if let Some(deposit_receipt_version) = self.receipt.deposit_receipt_version {
                deposit_receipt_version.encode(out)
            }
        }
    }

    fn encode_inner(&self, out: &mut dyn BufMut, with_header: bool) {
        if matches!(self.receipt.tx_type, TxType::Legacy) {
            self.encode_fields(out);
            return;
        }

        let mut payload = BytesMut::new();
        self.encode_fields(&mut payload);

        if with_header {
            let payload_length = payload.len() + 1;
            let header = alloy_rlp::Header {
                list: false,
                payload_length,
            };
            header.encode(out);
        }

        match self.receipt.tx_type {
            TxType::Legacy => unreachable!("legacy already handled"),

            TxType::Eip2930 => {
                out.put_u8(0x01);
            }
            TxType::Eip1559 => {
                out.put_u8(0x02);
            }
            TxType::Eip4844 => {
                out.put_u8(0x03);
            }
            TxType::Deposit => {
                out.put_u8(0x7E);
            }
        }
        out.put_slice(payload.as_ref());
    }

    /// Decodes the receipt payload
    fn decode_receipt(buf: &mut &[u8], tx_type: TxType) -> alloy_rlp::Result<Self> {
        let b: &mut &[u8] = &mut &**buf;
        let rlp_head = alloy_rlp::Header::decode(b)?;
        if !rlp_head.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }
        let started_len = b.len();

        let success = Decodable::decode(b)?;
        let cumulative_gas_used = Decodable::decode(b)?;
        let bloom = Decodable::decode(b)?;
        let logs = Decodable::decode(b)?;

        let receipt = match tx_type {
            TxType::Deposit => {
                let remaining = |b: &[u8]| rlp_head.payload_length - (started_len - b.len()) > 0;
                let deposit_nonce = remaining(b)
                    .then(|| alloy_rlp::Decodable::decode(b))
                    .transpose()?;
                let deposit_receipt_version = remaining(b)
                    .then(|| alloy_rlp::Decodable::decode(b))
                    .transpose()?;

                Receipt {
                    tx_type,
                    success,
                    cumulative_gas_used,
                    logs,
                    deposit_nonce,
                    deposit_receipt_version,
                }
            }
            _ => Receipt {
                tx_type,
                success,
                cumulative_gas_used,
                logs,
                deposit_nonce: None,
                deposit_receipt_version: None,
            },
        };

        let this = Self { receipt, bloom };
        let consumed = started_len - b.len();
        if consumed != rlp_head.payload_length {
            return Err(alloy_rlp::Error::ListLengthMismatch {
                expected: rlp_head.payload_length,
                got: consumed,
            });
        }
        *buf = *b;
        Ok(this)
    }
}

impl alloy_rlp::Encodable for ReceiptWithBloom {
    fn encode(&self, out: &mut dyn BufMut) {
        self.encode_inner(out, true)
    }

    fn length(&self) -> usize {
        let rlp_head = self.receipt_rlp_header();
        let mut payload_len = length_of_length(rlp_head.payload_length) + rlp_head.payload_length;
        // account for eip-2718 type prefix and set the list
        if !matches!(self.receipt.tx_type, TxType::Legacy) {
            payload_len += 1;
            // we include a string header for typed receipts, so include the length here
            payload_len += length_of_length(payload_len);
        }

        payload_len
    }
}

impl alloy_rlp::Decodable for ReceiptWithBloom {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        // a receipt is either encoded as a string (non legacy) or a list (legacy).
        // We should not consume the buffer if we are decoding a legacy receipt, so let's
        // check if the first byte is between 0x80 and 0xbf.
        let rlp_type = *buf.first().ok_or(alloy_rlp::Error::Custom(
            "cannot decode a receipt from empty bytes",
        ))?;

        match rlp_type.cmp(&alloy_rlp::EMPTY_LIST_CODE) {
            Ordering::Less => {
                // strip out the string header
                let _header = alloy_rlp::Header::decode(buf)?;
                let receipt_type = *buf.first().ok_or(alloy_rlp::Error::Custom(
                    "typed receipt cannot be decoded from an empty slice",
                ))?;
                match receipt_type {
                    0x01 => {
                        buf.advance(1);
                        Self::decode_receipt(buf, TxType::Eip2930)
                    }
                    0x02 => {
                        buf.advance(1);
                        Self::decode_receipt(buf, TxType::Eip1559)
                    }
                    0x03 => {
                        buf.advance(1);
                        Self::decode_receipt(buf, TxType::Eip4844)
                    }
                    0x7E => {
                        buf.advance(1);
                        Self::decode_receipt(buf, TxType::Deposit)
                    }
                    _ => Err(alloy_rlp::Error::Custom("invalid receipt type")),
                }
            }
            Ordering::Equal => Err(alloy_rlp::Error::Custom(
                "an empty list is not a valid receipt encoding",
            )),
            Ordering::Greater => Self::decode_receipt(buf, TxType::Legacy),
        }
    }
}
