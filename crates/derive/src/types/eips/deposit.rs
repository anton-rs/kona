use crate::types::{
    network::{Signed, Transaction, TxKind},
    transaction::TxType,
};
use alloc::vec::Vec;
use alloy_primitives::{keccak256, Address, Bytes, ChainId, Signature, B256, U256};
use alloy_rlp::{
    length_of_length, Buf, BufMut, Decodable, Encodable, Error as DecodeError, Header,
    EMPTY_STRING_CODE,
};
use core::mem;

/// Deposit transactions, also known as deposits are initiated on L1, and executed on L2.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TxDeposit {
    /// Hash that uniquely identifies the source of the deposit.
    pub source_hash: B256,
    /// The address of the sender account.
    pub from: Address,
    /// The address of the recipient account, or the null (zero-length) address if the deposited
    /// transaction is a contract creation.
    pub to: TxKind,
    /// The ETH value to mint on L2.
    pub mint: Option<u128>,
    ///  The ETH value to send to the recipient account.
    pub value: U256,
    /// The gas limit for the L2 transaction.
    pub gas_limit: u64,
    /// Field indicating if this transaction is exempt from the L2 gas limit.
    pub is_system_transaction: bool,
    /// Input has two uses depending if transaction is Create or Call (if `to` field is None or
    /// Some).
    pub input: Bytes,
}

impl TxDeposit {
    /// Calculates a heuristic for the in-memory size of the [TxDeposit] transaction.
    #[inline]
    pub fn size(&self) -> usize {
        mem::size_of::<B256>() + // source_hash
        mem::size_of::<Address>() + // from
        self.to.size() + // to
        mem::size_of::<Option<u128>>() + // mint
        mem::size_of::<U256>() + // value
        mem::size_of::<u64>() + // gas_limit
        mem::size_of::<bool>() + // is_system_transaction
        self.input.len() // input
    }

    /// Decodes the inner [TxDeposit] fields from RLP bytes.
    ///
    /// NOTE: This assumes a RLP header has already been decoded, and _just_ decodes the following
    /// RLP fields in the following order:
    ///
    /// - `source_hash`
    /// - `from`
    /// - `to`
    /// - `mint`
    /// - `value`
    /// - `gas_limit`
    /// - `is_system_transaction`
    /// - `input`
    pub fn decode_inner(buf: &mut &[u8]) -> Result<Self, DecodeError> {
        Ok(Self {
            source_hash: Decodable::decode(buf)?,
            from: Decodable::decode(buf)?,
            to: Decodable::decode(buf)?,
            mint: if *buf.first().ok_or(DecodeError::InputTooShort)? == EMPTY_STRING_CODE {
                buf.advance(1);
                None
            } else {
                Some(Decodable::decode(buf)?)
            },
            value: Decodable::decode(buf)?,
            gas_limit: Decodable::decode(buf)?,
            is_system_transaction: Decodable::decode(buf)?,
            input: Decodable::decode(buf)?,
        })
    }

    /// Outputs the length of the transaction's fields, without a RLP header or length of the
    /// eip155 fields.
    pub(crate) fn fields_len(&self) -> usize {
        self.source_hash.length()
            + self.from.length()
            + self.to.length()
            + self.mint.map_or(1, |mint| mint.length())
            + self.value.length()
            + self.gas_limit.length()
            + self.is_system_transaction.length()
            + self.input.0.length()
    }

    /// Encodes only the transaction's fields into the desired buffer, without a RLP header.
    /// <https://github.com/ethereum-optimism/optimism/blob/develop/specs/deposits.md#the-deposited-transaction-type>
    pub(crate) fn encode_fields(&self, out: &mut dyn alloy_rlp::BufMut) {
        self.source_hash.encode(out);
        self.from.encode(out);
        self.to.encode(out);
        if let Some(mint) = self.mint {
            mint.encode(out);
        } else {
            out.put_u8(EMPTY_STRING_CODE);
        }
        self.value.encode(out);
        self.gas_limit.encode(out);
        self.is_system_transaction.encode(out);
        self.input.encode(out);
    }

    /// Inner encoding function that is used for both rlp [`Encodable`] trait and for calculating
    /// hash that for eip2718 does not require rlp header.
    ///
    /// NOTE: Deposit transactions are not signed, so this function does not encode a signature,
    /// just the header and transaction rlp.
    pub(crate) fn encode_with_signature(
        &self,
        signature: &Signature,
        out: &mut dyn alloy_rlp::BufMut,
    ) {
        let payload_length = self.fields_len();
        let header = Header {
            list: true,
            payload_length,
        };
        header.encode(out);
        self.encode_fields(out);
    }

    /// Output the length of the RLP signed transaction encoding. This encodes with a RLP header.
    pub(crate) fn payload_len(&self) -> usize {
        let payload_length = self.fields_len();
        // 'tx type' + 'header length' + 'payload length'
        let len = 1 + length_of_length(payload_length) + payload_length;
        length_of_length(len) + len
    }

    pub(crate) fn payload_len_without_header(&self) -> usize {
        let payload_length = self.fields_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    /// Get the transaction type
    pub(crate) fn tx_type(&self) -> TxType {
        TxType::Deposit
    }
}

impl Encodable for TxDeposit {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        Header {
            list: true,
            payload_length: self.fields_len(),
        }
        .encode(out);
        self.encode_fields(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.fields_len();
        length_of_length(payload_length) + payload_length
    }
}

impl Decodable for TxDeposit {
    fn decode(data: &mut &[u8]) -> alloy_rlp::Result<Self> {
        let header = Header::decode(data)?;
        let remaining_len = data.len();

        if header.payload_length > remaining_len {
            return Err(alloy_rlp::Error::InputTooShort);
        }

        Self::decode_inner(data)
    }
}

impl Transaction for TxDeposit {
    type Signature = Signature;

    fn encode_for_signing(&self, out: &mut dyn alloy_rlp::BufMut) {
        out.put_u8(self.tx_type() as u8);
        Header {
            list: true,
            payload_length: self.fields_len(),
        }
        .encode(out);
        self.encode_fields(out);
    }

    fn payload_len_for_signature(&self) -> usize {
        let payload_length = self.fields_len();
        // 'transaction type byte length' + 'header length' + 'payload length'
        1 + length_of_length(payload_length) + payload_length
    }

    fn into_signed(self, signature: Signature) -> Signed<Self> {
        let payload_length = 1 + self.fields_len() + signature.rlp_vrs_len();
        let mut buf = Vec::with_capacity(payload_length);
        buf.put_u8(TxType::Eip1559 as u8);
        self.encode_signed(&signature, &mut buf);
        let hash = keccak256(&buf);

        // Drop any v chain id value to ensure the signature format is correct at the time of
        // combination for an EIP-1559 transaction. V should indicate the y-parity of the
        // signature.
        Signed::new_unchecked(self, signature.with_parity_bool(), hash)
    }

    fn encode_signed(&self, signature: &Signature, out: &mut dyn alloy_rlp::BufMut) {
        TxDeposit::encode_with_signature(self, signature, out)
    }

    fn decode_signed(buf: &mut &[u8]) -> alloy_rlp::Result<Signed<Self>> {
        let header = Header::decode(buf)?;
        if !header.list {
            return Err(alloy_rlp::Error::UnexpectedString);
        }

        let tx = Self::decode_inner(buf)?;
        let signature = Signature::decode_rlp_vrs(buf)?;

        Ok(tx.into_signed(signature))
    }

    fn input(&self) -> &[u8] {
        &self.input
    }

    fn input_mut(&mut self) -> &mut Bytes {
        &mut self.input
    }

    fn set_input(&mut self, input: Bytes) {
        self.input = input;
    }

    fn to(&self) -> TxKind {
        self.to
    }

    fn set_to(&mut self, to: TxKind) {
        self.to = to;
    }

    fn value(&self) -> U256 {
        self.value
    }

    fn set_value(&mut self, value: U256) {
        self.value = value;
    }

    fn chain_id(&self) -> Option<ChainId> {
        None
    }

    fn set_chain_id(&mut self, chain_id: ChainId) {
        unreachable!("Deposit transactions do not have a chain id");
    }

    fn nonce(&self) -> u64 {
        0
    }

    fn set_nonce(&mut self, nonce: u64) {
        unreachable!("Deposit transactions do not have a nonce");
    }

    fn gas_limit(&self) -> u64 {
        self.gas_limit
    }

    fn set_gas_limit(&mut self, limit: u64) {
        self.gas_limit = limit;
    }

    fn gas_price(&self) -> Option<U256> {
        None
    }

    fn set_gas_price(&mut self, price: U256) {
        let _ = price;
    }
}
