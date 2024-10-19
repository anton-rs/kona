//! Contains utilities for the L2 executor.

use crate::{ExecutorError, ExecutorResult};
use alloc::vec::Vec;
use alloy_consensus::{Eip658Value, Header, Receipt, ReceiptWithBloom};
use alloy_eips::eip1559::BaseFeeParams;
use alloy_primitives::{Bloom, Bytes, Log, B64};
use op_alloy_consensus::{
    OpDepositReceipt, OpDepositReceiptWithBloom, OpReceiptEnvelope, OpTxType,
};
use op_alloy_genesis::RollupConfig;
use op_alloy_rpc_types_engine::OpPayloadAttributes;

/// The version byte for the Holocene extra data.
const HOLOCENE_EXTRA_DATA_VERSION: u8 = 0x00;

/// Constructs a [OpReceiptEnvelope] from a [Receipt] fields and [OpTxType].
pub(crate) fn receipt_envelope_from_parts<'a>(
    status: bool,
    cumulative_gas_used: u128,
    logs: impl IntoIterator<Item = &'a Log>,
    tx_type: OpTxType,
    deposit_nonce: Option<u64>,
    deposit_receipt_version: Option<u64>,
) -> OpReceiptEnvelope {
    let logs = logs.into_iter().cloned().collect::<Vec<_>>();
    let logs_bloom = logs_bloom(&logs);
    let inner_receipt = Receipt { status: Eip658Value::Eip658(status), cumulative_gas_used, logs };
    match tx_type {
        OpTxType::Legacy => {
            OpReceiptEnvelope::Legacy(ReceiptWithBloom { receipt: inner_receipt, logs_bloom })
        }
        OpTxType::Eip2930 => {
            OpReceiptEnvelope::Eip2930(ReceiptWithBloom { receipt: inner_receipt, logs_bloom })
        }
        OpTxType::Eip1559 => {
            OpReceiptEnvelope::Eip1559(ReceiptWithBloom { receipt: inner_receipt, logs_bloom })
        }
        OpTxType::Eip7702 => panic!("EIP-7702 is not supported"),
        OpTxType::Deposit => {
            let inner = OpDepositReceiptWithBloom {
                receipt: OpDepositReceipt {
                    inner: inner_receipt,
                    deposit_nonce,
                    deposit_receipt_version,
                },
                logs_bloom,
            };
            OpReceiptEnvelope::Deposit(inner)
        }
    }
}

/// Compute the logs bloom filter for the given logs.
pub(crate) fn logs_bloom<'a>(logs: impl IntoIterator<Item = &'a Log>) -> Bloom {
    let mut bloom = Bloom::ZERO;
    for log in logs {
        bloom.m3_2048(log.address.as_slice());
        for topic in log.topics() {
            bloom.m3_2048(topic.as_slice());
        }
    }
    bloom
}

/// Parse Holocene [Header] extra data.
///
/// ## Takes
/// - `extra_data`: The extra data field of the [Header].
///
/// ## Returns
/// - `Ok(BaseFeeParams)`: The EIP-1559 parameters.
/// - `Err(ExecutorError::InvalidExtraData)`: If the extra data is invalid.
pub(crate) fn decode_holocene_eip_1559_params(header: &Header) -> ExecutorResult<BaseFeeParams> {
    // Check the extra data length.
    if header.extra_data.len() != 1 + 8 {
        return Err(ExecutorError::InvalidExtraData);
    }

    // Check the extra data version byte.
    if header.extra_data[0] != HOLOCENE_EXTRA_DATA_VERSION {
        return Err(ExecutorError::InvalidExtraData);
    }

    // Parse the EIP-1559 parameters.
    let data = &header.extra_data[1..];
    let denominator =
        u32::from_be_bytes(data[..4].try_into().map_err(|_| ExecutorError::InvalidExtraData)?)
            as u128;
    let elasticity =
        u32::from_be_bytes(data[4..].try_into().map_err(|_| ExecutorError::InvalidExtraData)?)
            as u128;

    // Check for potential division by zero.
    if denominator == 0 {
        return Err(ExecutorError::InvalidExtraData);
    }

    Ok(BaseFeeParams { elasticity_multiplier: elasticity, max_change_denominator: denominator })
}

/// Encode Holocene [Header] extra data.
///
/// ## Takes
/// - `config`: The [RollupConfig] for the chain.
/// - `attributes`: The [OpPayloadAttributes] for the block.
///
/// ## Returns
/// - `Ok(data)`: The encoded extra data.
/// - `Err(ExecutorError::MissingEIP1559Params)`: If the EIP-1559 parameters are missing.
pub(crate) fn encode_holocene_eip_1559_params(
    config: &RollupConfig,
    attributes: &OpPayloadAttributes,
) -> ExecutorResult<Bytes> {
    let payload_params = attributes.eip_1559_params.ok_or(ExecutorError::MissingEIP1559Params)?;
    let params = if payload_params == B64::ZERO {
        encode_canyon_base_fee_params(config)
    } else {
        payload_params
    };

    let mut data = Vec::with_capacity(1 + 8);
    data.push(HOLOCENE_EXTRA_DATA_VERSION);
    data.extend_from_slice(params.as_ref());
    Ok(data.into())
}

/// Encodes the canyon base fee parameters, per Holocene spec.
///
/// <https://specs.optimism.io/protocol/holocene/exec-engine.html#eip1559params-encoding>
pub(crate) fn encode_canyon_base_fee_params(config: &RollupConfig) -> B64 {
    let params = config.canyon_base_fee_params;

    let mut buf = B64::ZERO;
    buf[..4].copy_from_slice(&(params.max_change_denominator as u32).to_be_bytes());
    buf[4..].copy_from_slice(&(params.elasticity_multiplier as u32).to_be_bytes());
    buf
}

#[cfg(test)]
mod test {
    use super::decode_holocene_eip_1559_params;
    use crate::util::{encode_canyon_base_fee_params, encode_holocene_eip_1559_params};
    use alloy_consensus::Header;
    use alloy_eips::eip1559::BaseFeeParams;
    use alloy_primitives::{b64, hex, B64};
    use alloy_rpc_types_engine::PayloadAttributes;
    use op_alloy_genesis::RollupConfig;
    use op_alloy_rpc_types_engine::OpPayloadAttributes;

    fn mock_payload(eip_1559_params: Option<B64>) -> OpPayloadAttributes {
        OpPayloadAttributes {
            payload_attributes: PayloadAttributes {
                timestamp: 0,
                prev_randao: Default::default(),
                suggested_fee_recipient: Default::default(),
                withdrawals: Default::default(),
                parent_beacon_block_root: Default::default(),
            },
            transactions: None,
            no_tx_pool: None,
            gas_limit: None,
            eip_1559_params,
        }
    }

    #[test]
    fn test_decode_holocene_eip_1559_params() {
        let params = hex!("00BEEFBABE0BADC0DE");
        let mock_header = Header { extra_data: params.to_vec().into(), ..Default::default() };
        let params = decode_holocene_eip_1559_params(&mock_header).unwrap();

        assert_eq!(params.elasticity_multiplier, 0x0BAD_C0DE);
        assert_eq!(params.max_change_denominator, 0xBEEF_BABE);
    }

    #[test]
    fn test_decode_holocene_eip_1559_params_invalid_version() {
        let params = hex!("01BEEFBABE0BADC0DE");
        let mock_header = Header { extra_data: params.to_vec().into(), ..Default::default() };
        assert!(decode_holocene_eip_1559_params(&mock_header).is_err());
    }

    #[test]
    fn test_decode_holocene_eip_1559_params_invalid_denominator() {
        let params = hex!("00000000000BADC0DE");
        let mock_header = Header { extra_data: params.to_vec().into(), ..Default::default() };
        assert!(decode_holocene_eip_1559_params(&mock_header).is_err());
    }

    #[test]
    fn test_decode_holocene_eip_1559_params_invalid_length() {
        let params = hex!("00");
        let mock_header = Header { extra_data: params.to_vec().into(), ..Default::default() };
        assert!(decode_holocene_eip_1559_params(&mock_header).is_err());
    }

    #[test]
    fn test_encode_holocene_eip_1559_params_missing() {
        let cfg = RollupConfig {
            canyon_base_fee_params: BaseFeeParams {
                max_change_denominator: 32,
                elasticity_multiplier: 64,
            },
            ..Default::default()
        };
        let attrs = mock_payload(None);

        assert!(encode_holocene_eip_1559_params(&cfg, &attrs).is_err());
    }

    #[test]
    fn test_encode_holocene_eip_1559_params_default() {
        let cfg = RollupConfig {
            canyon_base_fee_params: BaseFeeParams {
                max_change_denominator: 32,
                elasticity_multiplier: 64,
            },
            ..Default::default()
        };
        let attrs = mock_payload(Some(B64::ZERO));

        assert_eq!(
            encode_holocene_eip_1559_params(&cfg, &attrs).unwrap(),
            hex!("000000002000000040").to_vec()
        );
    }

    #[test]
    fn test_encode_holocene_eip_1559_params() {
        let cfg = RollupConfig {
            canyon_base_fee_params: BaseFeeParams {
                max_change_denominator: 32,
                elasticity_multiplier: 64,
            },
            ..Default::default()
        };
        let attrs = mock_payload(Some(b64!("0000004000000060")));

        assert_eq!(
            encode_holocene_eip_1559_params(&cfg, &attrs).unwrap(),
            hex!("000000004000000060").to_vec()
        );
    }

    #[test]
    fn test_encode_canyon_1559_params() {
        let cfg = RollupConfig {
            canyon_base_fee_params: BaseFeeParams {
                max_change_denominator: 32,
                elasticity_multiplier: 64,
            },
            ..Default::default()
        };
        assert_eq!(encode_canyon_base_fee_params(&cfg), b64!("0000002000000040"));
    }
}
