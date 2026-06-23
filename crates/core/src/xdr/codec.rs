

use crate::error::{PrismError, PrismResult};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use stellar_xdr::curr::{
    ContractEvent, DiagnosticEvent, LedgerEntry, Limits, ReadXdr, ScVal, ScVec,
    TransactionEnvelope, TransactionMeta, WriteXdr, TransactionResult,
};

pub trait XdrCodec: Sized {

    const TYPE_NAME: &'static str;

    /// Decode from XDR bytes.
    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self>;

    /// Encode to XDR bytes.
    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>>;

    /// Decode from a base64-encoded XDR string.
    fn from_xdr_base64(b64: &str) -> PrismResult<Self> {
        let bytes = decode_xdr_base64(b64)?;
        Self::from_xdr_bytes(&bytes)
    }

    /// Encode to a base64-encoded XDR string.
    fn to_xdr_base64(&self) -> PrismResult<String> {
        let bytes = self.to_xdr_bytes()?;
        Ok(encode_xdr_base64(&bytes))
    }
}

impl XdrCodec for TransactionMeta {
    const TYPE_NAME: &'static str = "TransactionMeta";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        TransactionMeta::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for TransactionEnvelope {
    const TYPE_NAME: &'static str = "TransactionEnvelope";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        TransactionEnvelope::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for TransactionResult {
    const TYPE_NAME: &'static str = "TransactionResult";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        TransactionResult::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for LedgerEntry {
    const TYPE_NAME: &'static str = "LedgerEntry";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        LedgerEntry::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for DiagnosticEvent {
    const TYPE_NAME: &'static str = "DiagnosticEvent";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        DiagnosticEvent::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for ScVec {
    const TYPE_NAME: &'static str = "ScVec";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        ScVec::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for ContractEvent {
    const TYPE_NAME: &'static str = "ContractEvent";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        ContractEvent::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

impl XdrCodec for ScVal {
    const TYPE_NAME: &'static str = "ScVal";

    fn from_xdr_bytes(bytes: &[u8]) -> PrismResult<Self> {
        ScVal::from_xdr(bytes, Limits::none()).map_err(|e| {
            PrismError::XdrDecodingFailed {
                type_name: Self::TYPE_NAME,
                reason: e.to_string(),
            }
        })
    }

    fn to_xdr_bytes(&self) -> PrismResult<Vec<u8>> {
        self.to_xdr(Limits::none()).map_err(|e| {
            PrismError::XdrError(format!("Failed to encode {}: {}", Self::TYPE_NAME, e))
        })
    }
}

/// Decode a base64-encoded XDR string to raw bytes.
pub fn decode_xdr_base64(xdr_base64: &str) -> PrismResult<Vec<u8>> {
    STANDARD.decode(xdr_base64).map_err(|e| {
        PrismError::XdrError(format!("Base64 decode failed: {e}"))
    })
}

/// Encode raw bytes to a base64 XDR string.
pub fn encode_xdr_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

/// Decode a transaction hash from hex string.
pub fn decode_tx_hash(hash_hex: &str) -> PrismResult<[u8; 32]> {
    let bytes = hex_decode(hash_hex)
        .map_err(|e| PrismError::XdrError(format!("Invalid tx hash hex: {e}")))?;

    if bytes.len() != 32 {
        return Err(PrismError::XdrError(format!(
            "Transaction hash must be 32 bytes, got {}",
            bytes.len()
        )));
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn hex_decode(input: &str) -> Result<Vec<u8>, String> {
    if !input.len().is_multiple_of(2) {
        return Err("Hex input must have an even length".to_string());
    }

    (0..input.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&input[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at position {i}: {e}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{
        ExtensionPoint, Memo, MuxedAccount, OperationMeta, Preconditions, ScMapEntry, ScVal,
        SequenceNumber, Transaction, TransactionExt, TransactionMetaV3, TransactionV1Envelope,
        Uint256,
    };

    fn make_test_envelope() -> TransactionEnvelope {
        TransactionEnvelope::Tx(TransactionV1Envelope {
            tx: Transaction {
                source_account: MuxedAccount::Ed25519(Uint256([0; 32])),
                fee: 100,
                seq_num: SequenceNumber(1),
                cond: Preconditions::None,
                memo: Memo::None,
                operations: vec![].try_into().unwrap(),
                ext: TransactionExt::V0,
            },
            signatures: vec![].try_into().unwrap(),
        })
    }

    #[test]
    fn test_xdr_codec_round_trip() {
        let envelope = make_test_envelope();
        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&envelope).expect("encode");
        let decoded = <TransactionEnvelope as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(envelope, decoded);
    }

    #[test]
    fn test_transaction_meta_v3_decoding() {
        let meta = TransactionMeta::V3(TransactionMetaV3 {
            ext: ExtensionPoint::V0,
            tx_changes_before: vec![].try_into().expect("empty changes"),
            operations: vec![OperationMeta {
                changes: vec![].try_into().expect("empty operation changes"),
            }]
            .try_into()
            .expect("one operation"),
            tx_changes_after: vec![].try_into().expect("empty changes"),
            soroban_meta: Some(stellar_xdr::curr::SorobanTransactionMeta {
                ext: stellar_xdr::curr::SorobanTransactionMetaExt::V0,
                events: vec![stellar_xdr::curr::ContractEvent {
                    ext: ExtensionPoint::V0,
                    contract_id: None,
                    type_: stellar_xdr::curr::ContractEventType::Contract,
                    body: stellar_xdr::curr::ContractEventBody::V0(stellar_xdr::curr::ContractEventV0 {
                        topics: vec![].try_into().unwrap(),
                        data: stellar_xdr::curr::ScVal::Void,
                    }),
                }].try_into().unwrap(),
                return_value: stellar_xdr::curr::ScVal::Void,
                diagnostic_events: vec![].try_into().unwrap(),
            }),
        });

        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&meta).expect("encode V3");
        let decoded = <TransactionMeta as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode V3");

        if let TransactionMeta::V3(v3) = decoded {
            assert_eq!(v3.operations.len(), 1);
            let soroban = v3.soroban_meta.expect("soroban_meta");
            assert_eq!(soroban.events.len(), 1);
        } else {
            panic!("expected V3");
        }
    }

    #[test]
    fn test_decode_tx_hash_valid() {
        let hash = "a".repeat(64);
        assert!(decode_tx_hash(&hash).is_ok());
    }

    #[test]
    fn test_transaction_result_round_trip() {
        let xdr_bytes = vec![0u8; 20];
        let bytes = encode_xdr_base64(&xdr_bytes);

        let decoded = <TransactionResult as crate::xdr::codec::XdrCodec>::from_xdr_base64(&bytes).expect("decode");
        let encoded = crate::xdr::codec::XdrCodec::to_xdr_base64(&decoded).expect("encode");

        assert_eq!(bytes, encoded);
    }

    #[test]
    fn test_diagnostic_event_round_trip() {
        let event = DiagnosticEvent {
            in_successful_contract_call: true,
            event: stellar_xdr::curr::ContractEvent {
                ext: ExtensionPoint::V0,
                contract_id: None,
                type_: stellar_xdr::curr::ContractEventType::Contract,
                body: stellar_xdr::curr::ContractEventBody::V0(stellar_xdr::curr::ContractEventV0 {
                    topics: vec![].try_into().unwrap(),
                    data: stellar_xdr::curr::ScVal::Void,
                }),
            },
        };

        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&event).expect("encode");
        let decoded = <DiagnosticEvent as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(event, decoded);
    }

    #[test]
    fn test_scvec_round_trip() {
        let scvec = ScVec(vec![
            stellar_xdr::curr::ScVal::Void,
            stellar_xdr::curr::ScVal::Bool(true),
            stellar_xdr::curr::ScVal::U32(42),
        ].try_into().unwrap());

        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&scvec).expect("encode");
        let decoded = <ScVec as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(scvec, decoded);
    }

    #[test]
    fn test_scmap_empty_round_trip() {
        let map = ScMap(vec![].try_into().unwrap());
        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&map).expect("encode");
        let decoded = <ScMap as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(map, decoded);
        assert_eq!(decoded.0.len(), 0);
    }

    #[test]
    fn test_scmap_single_entry_round_trip() {
        let entry = ScMapEntry {
            key: ScVal::U32(1),
            val: ScVal::Bool(true),
        };
        // ScMapEntry round-trip
        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&entry).expect("encode");
        let decoded_entry = <ScMapEntry as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(entry, decoded_entry);

        // ScMap with single entry
        let map = ScMap::sorted_from_entries(vec![ScMapEntry {
            key: ScVal::U32(1),
            val: ScVal::Bool(true),
        }].into_iter()).expect("sorted_from_entries");
        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&map).expect("encode");
        let decoded = <ScMap as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(map, decoded);
        assert_eq!(decoded.0.len(), 1);
        assert_eq!(decoded.0[0].key, ScVal::U32(1));
        assert_eq!(decoded.0[0].val, ScVal::Bool(true));
    }

    #[test]
    fn test_scmap_multi_entry_round_trip() {
        // Entries inserted out of order — sorted_from_entries must sort them by key.
        let entries = vec![
            ScMapEntry { key: ScVal::U32(3), val: ScVal::Bool(false) },
            ScMapEntry { key: ScVal::U32(1), val: ScVal::Void },
            ScMapEntry { key: ScVal::U32(2), val: ScVal::Bool(true) },
        ];
        let map = ScMap::sorted_from_entries(entries.into_iter()).expect("sorted_from_entries");
        let b64 = crate::xdr::codec::XdrCodec::to_xdr_base64(&map).expect("encode");
        let decoded = <ScMap as crate::xdr::codec::XdrCodec>::from_xdr_base64(&b64).expect("decode");
        assert_eq!(map, decoded);
        assert_eq!(decoded.0.len(), 3);
        // Keys must be in ascending order after sorting.
        assert_eq!(decoded.0[0].key, ScVal::U32(1));
        assert_eq!(decoded.0[1].key, ScVal::U32(2));
        assert_eq!(decoded.0[2].key, ScVal::U32(3));
        assert_eq!(decoded.0[0].val, ScVal::Void);
        assert_eq!(decoded.0[1].val, ScVal::Bool(true));
        assert_eq!(decoded.0[2].val, ScVal::Bool(false));
    }
}
