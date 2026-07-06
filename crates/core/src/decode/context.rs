

use crate::decode::auth::{AuthChain, AuthCredential};
use crate::decode::auth_signature::decode_auth_entry_signatures;
use crate::error::PrismResult;
use crate::types::report::{AuthEntryInfo, DiagnosticReport, FeeBreakdown, ResourceSummary, TransactionContext};
use crate::xdr::codec::XdrCodec;
use stellar_xdr::curr::{TransactionEnvelope, TransactionMeta, TransactionResult};

pub fn enrich_report(
    report: &mut DiagnosticReport,
    tx_data: &serde_json::Value,
) -> PrismResult<()> {
    let tx_hash = tx_data
        .get("hash")
        .and_then(|h| h.as_str())
        .unwrap_or("unknown")
        .to_string();

    let ledger_sequence = tx_data.get("ledger").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32;

    let context = TransactionContext {
        tx_hash,
        ledger_sequence,
        function_name: extract_function_name(tx_data),
        arguments: extract_arguments(tx_data),
        return_value: extract_return_value(tx_data),
        fee: extract_fee_breakdown(tx_data),
        resources: extract_resource_summary(tx_data),
    };

    report.transaction_context = Some(context);

    // Decode ed25519 signatures from auth entries embedded in the transaction envelope.
    report.auth_signatures = extract_auth_signatures(tx_data);

    // Build typed auth entry summaries (Ed25519 vs Smart Wallet).
    report.auth_entries = extract_auth_entries(tx_data);

    Ok(())
}

fn extract_function_name(tx_data: &serde_json::Value) -> Option<String> {
    tx_data
        .get("functionName")
        .and_then(|f| f.as_str())
        .map(std::string::ToString::to_string)
}

fn extract_arguments(tx_data: &serde_json::Value) -> Vec<String> {
    tx_data
        .get("arguments")
        .and_then(|a| a.as_array())
        .map(|args| args.iter().map(std::string::ToString::to_string).collect())
        .unwrap_or_default()
}

fn extract_return_value(tx_data: &serde_json::Value) -> Option<String> {
    tx_data
        .get("returnValue")
        .and_then(|r| r.as_str())
        .map(std::string::ToString::to_string)
}

fn extract_fee_breakdown(tx_data: &serde_json::Value) -> FeeBreakdown {
    // 1. Get total fee from resultXdr
    let mut total_fee = 0;
    if let Some(result_xdr_b64) = tx_data.get("resultXdr").and_then(|v| v.as_str()) {
        if let Ok(tx_result) = TransactionResult::from_xdr_base64(result_xdr_b64) {
            total_fee = tx_result.fee_charged;
        }
    }

    // 2. Get bid fee from envelopeXdr
    let mut bid_fee = None;
    if let Some(envelope_xdr_b64) = tx_data.get("envelopeXdr").and_then(|v| v.as_str()) {
        if let Ok(tx_envelope) = TransactionEnvelope::from_xdr_base64(envelope_xdr_b64) {
            match tx_envelope {
                TransactionEnvelope::Tx(v1) => {
                    bid_fee = Some(v1.tx.fee as i64);
                }
                TransactionEnvelope::TxFeeBump(fee_bump) => {
                    bid_fee = Some(fee_bump.tx.fee as i64);
                }
                TransactionEnvelope::TxV0(v0) => {
                    bid_fee = Some(v0.tx.fee as i64);
                }
            }
        }
    }

    // 3. Get resource fee components from resultMetaXdr or the pre-parsed fee payload.
    let mut non_refundable_fee = 0;
    let mut refundable_fee = 0;
    let mut rent_fee = 0;
    let mut has_soroban_meta = false;

    if let Some(resource_fee_obj) = tx_data.get("resourceFee").and_then(|v| v.as_object()) {
        non_refundable_fee = resource_fee_obj
            .get("totalNonRefundableResourceFeeCharged")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        refundable_fee = resource_fee_obj
            .get("totalRefundableResourceFeeCharged")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        rent_fee = resource_fee_obj
            .get("rentFeeCharged")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        has_soroban_meta = true;
    } else if let Some(meta_xdr_b64) = tx_data.get("resultMetaXdr").and_then(|v| v.as_str()) {
        if let Ok(tx_meta) = TransactionMeta::from_xdr_base64(meta_xdr_b64) {
            match tx_meta {
                TransactionMeta::V3(v3) => {
                    if let Some(soroban_meta) = v3.soroban_meta {
                        match soroban_meta.ext {
                            stellar_xdr::curr::SorobanTransactionMetaExt::V0 => {}
                            stellar_xdr::curr::SorobanTransactionMetaExt::V1(v1) => {
                                non_refundable_fee = v1.total_non_refundable_resource_fee_charged;
                                refundable_fee = v1.total_refundable_resource_fee_charged;
                                rent_fee = v1.rent_fee_charged;
                                has_soroban_meta = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let resource_fee = if has_soroban_meta {
        non_refundable_fee + refundable_fee + rent_fee
    } else {
        0
    };

    let inclusion_fee = tx_data
        .get("inclusionFee")
        .and_then(|v| v.as_i64())
        .unwrap_or(total_fee - resource_fee);

    FeeBreakdown {
        total_charged_fee: total_fee,
        inclusion_fee,
        resource_fee,
        refundable_resource_fee: refundable_fee,
        refundable_fee: refundable_fee + rent_fee,
        non_refundable_fee,
        bid_fee,
    }
}

fn extract_resource_summary(tx_data: &serde_json::Value) -> ResourceSummary {
    let mut cpu_used = 0;
    let mut cpu_limit = 0;
    let mut mem_used = 0;
    let mut mem_limit = 0;
    let mut read_used = 0;
    let mut read_limit = 0;

    if let Some(events) = tx_data.get("diagnosticEvents").and_then(|e| e.as_array()) {
        for event in events {
            if event.get("type").and_then(|t| t.as_str()) == Some("budget") {
                if let Some(data) = event.get("data") {
                    let category = data.get("category").and_then(|c| c.as_str()).unwrap_or("");
                    let used = data.get("used").and_then(|u| u.as_u64()).unwrap_or(0);
                    let limit = data.get("limit").and_then(|l| l.as_u64()).unwrap_or(0);

                    if category == "cpu" {
                        cpu_used = used;
                        cpu_limit = limit;
                    } else if category == "memory" {
                        mem_used = used;
                        mem_limit = limit;
                    } else if category == "read" {
                        read_used = used;
                        read_limit = limit;
                    }
                }
            }
        }
    }

    ResourceSummary {
        cpu_instructions_used: cpu_used,
        cpu_instructions_limit: cpu_limit,
        memory_bytes_used: mem_used,
        memory_bytes_limit: mem_limit,
        read_bytes: read_used,
        read_bytes_limit: read_limit,
        write_bytes: 0,
    }
}

/// Extract and decode ed25519 signatures from auth entries in the transaction envelope.
/// Auth entries are base64 XDR strings found under `tx.operations[*].body.invoke_host_function_op.auth`
/// or directly in the `auth` field of an RPC simulate response stored in tx_data.
fn extract_auth_signatures(tx_data: &serde_json::Value) -> Vec<String> {
    let mut signatures = Vec::new();

    // Auth entries may appear directly as a top-level "auth" array (simulate response shape)
    // or nested inside envelopeXdr operations.
    if let Some(auth_array) = tx_data.get("auth").and_then(|a| a.as_array()) {
        for entry in auth_array {
            if let Some(xdr_b64) = entry.as_str() {
                signatures.extend(decode_auth_entry_signatures(xdr_b64));
            }
        }
    }

    signatures
}

/// Build typed summaries for each auth entry found in the transaction.
///
/// For each entry, an [`AuthEntryInfo`] is produced that labels it as either
/// Ed25519 or Smart Wallet and surfaces the relevant address / contract ID.
/// Entries that cannot be decoded are silently skipped to remain non-breaking.
fn extract_auth_entries(tx_data: &serde_json::Value) -> Vec<AuthEntryInfo> {
    let mut entries = Vec::new();

    if let Some(auth_array) = tx_data.get("auth").and_then(|a| a.as_array()) {
        for entry in auth_array {
            if let Some(xdr_b64) = entry.as_str() {
                if let Ok(chain) = AuthChain::from_xdr_base64(xdr_b64) {
                    if let Some(info) = auth_entry_info_from_chain(&chain) {
                        entries.push(info);
                    }
                }
            }
        }
    }

    entries
}

/// Convert an [`AuthChain`] credential into an [`AuthEntryInfo`] label.
/// Returns `None` for `SourceAccount` credentials, which carry no address info.
fn auth_entry_info_from_chain(chain: &AuthChain) -> Option<AuthEntryInfo> {
    match &chain.credential {
        AuthCredential::SourceAccount => None,
        AuthCredential::Address(cred) => Some(AuthEntryInfo {
            auth_type: cred.auth_type.to_string(),
            address: cred.address.clone(),
            contract_id: cred.contract_id.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xdr::codec::XdrCodec;
    use stellar_xdr::curr::{
        Memo, MuxedAccount, Preconditions, SequenceNumber, Transaction, TransactionEnvelope,
        TransactionExt, TransactionResult, TransactionResultResult, TransactionV1Envelope, Uint256,
        TransactionMeta, TransactionMetaV3, SorobanTransactionMeta, SorobanTransactionMetaExt,
        SorobanTransactionMetaExtV1, ExtensionPoint,
    };

    #[test]
    fn test_extract_resource_summary_read() {
        let tx_data = serde_json::json!({
            "diagnosticEvents": [
                {
                    "type": "budget",
                    "data": {
                        "category": "read",
                        "used": 12345,
                        "limit": 100000
                    }
                }
            ]
        });
        let result = extract_resource_summary(&tx_data);
        assert_eq!(result.read_bytes, 12345);
        assert_eq!(result.read_bytes_limit, 100000);
        assert_eq!(result.cpu_instructions_used, 0);
        assert_eq!(result.memory_bytes_used, 0);
    }

    #[test]
    fn test_extract_resource_summary_empty() {
        let tx_data = serde_json::json!({});
        let result = extract_resource_summary(&tx_data);
        assert_eq!(result.read_bytes, 0);
        assert_eq!(result.read_bytes_limit, 0);
        assert_eq!(result.cpu_instructions_used, 0);
        assert_eq!(result.memory_bytes_used, 0);
    }

    #[test]
    fn test_extract_resource_summary_cpu_regression() {
        let tx_data = serde_json::json!({
            "diagnosticEvents": [
                {
                    "type": "budget",
                    "data": {
                        "category": "cpu",
                        "used": 5000,
                        "limit": 10000
                    }
                }
            ]
        });
        let result = extract_resource_summary(&tx_data);
        assert_eq!(result.cpu_instructions_used, 5000);
        assert_eq!(result.cpu_instructions_limit, 10000);
        assert_eq!(result.read_bytes, 0);
        assert_eq!(result.read_bytes_limit, 0);
    }

    #[test]
    fn test_extract_resource_summary_unknown_category() {
        let tx_data = serde_json::json!({
            "diagnosticEvents": [
                {
                    "type": "budget",
                    "data": {
                        "category": "unknown_category",
                        "used": 999,
                        "limit": 9999
                    }
                }
            ]
        });
        let result = extract_resource_summary(&tx_data);
        assert_eq!(result.read_bytes, 0);
        assert_eq!(result.read_bytes_limit, 0);
        assert_eq!(result.cpu_instructions_used, 0);
        assert_eq!(result.memory_bytes_used, 0);
    }

    #[test]
    fn test_extract_fee_breakdown_non_soroban() {
        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256([0; 32])),
            fee: 150,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![].try_into().unwrap(),
            ext: TransactionExt::V0,
        };
        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: vec![].try_into().unwrap(),
        });
        let envelope_xdr = envelope.to_xdr_base64().unwrap();

        let result = TransactionResult {
            fee_charged: 120,
            result: TransactionResultResult::TxSuccess(vec![].try_into().unwrap()),
            ext: stellar_xdr::curr::TransactionResultExt::V0,
        };
        let result_xdr = result.to_xdr_base64().unwrap();

        let tx_data = serde_json::json!({
            "envelopeXdr": envelope_xdr,
            "resultXdr": result_xdr,
        });

        let breakdown = extract_fee_breakdown(&tx_data);
        assert_eq!(breakdown.total_charged_fee, 120);
        assert_eq!(breakdown.bid_fee, Some(150));
        assert_eq!(breakdown.inclusion_fee, 120);
        assert_eq!(breakdown.resource_fee, 0);
        assert_eq!(breakdown.refundable_resource_fee, 0);
        assert_eq!(breakdown.refundable_fee, 0);
        assert_eq!(breakdown.non_refundable_fee, 0);
    }

    #[test]
    fn test_extract_fee_breakdown_soroban() {
        let tx = Transaction {
            source_account: MuxedAccount::Ed25519(Uint256([0; 32])),
            fee: 500,
            seq_num: SequenceNumber(1),
            cond: Preconditions::None,
            memo: Memo::None,
            operations: vec![].try_into().unwrap(),
            ext: TransactionExt::V0,
        };
        let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
            tx,
            signatures: vec![].try_into().unwrap(),
        });
        let envelope_xdr = envelope.to_xdr_base64().unwrap();

        let result = TransactionResult {
            fee_charged: 450,
            result: TransactionResultResult::TxSuccess(vec![].try_into().unwrap()),
            ext: stellar_xdr::curr::TransactionResultExt::V0,
        };
        let result_xdr = result.to_xdr_base64().unwrap();

        let meta = TransactionMeta::V3(TransactionMetaV3 {
            ext: ExtensionPoint::V0,
            tx_changes_before: vec![].try_into().unwrap(),
            operations: vec![].try_into().unwrap(),
            tx_changes_after: vec![].try_into().unwrap(),
            soroban_meta: Some(SorobanTransactionMeta {
                ext: SorobanTransactionMetaExt::V1(SorobanTransactionMetaExtV1 {
                    ext: ExtensionPoint::V0,
                    total_non_refundable_resource_fee_charged: 100,
                    total_refundable_resource_fee_charged: 200,
                    rent_fee_charged: 50,
                }),
                events: vec![].try_into().unwrap(),
                return_value: stellar_xdr::curr::ScVal::Void,
                diagnostic_events: vec![].try_into().unwrap(),
            }),
        });
        let meta_xdr = meta.to_xdr_base64().unwrap();

        let tx_data = serde_json::json!({
            "envelopeXdr": envelope_xdr,
            "resultXdr": result_xdr,
            "resultMetaXdr": meta_xdr,
        });

        let breakdown = extract_fee_breakdown(&tx_data);
        assert_eq!(breakdown.total_charged_fee, 450);
        assert_eq!(breakdown.bid_fee, Some(500));
        assert_eq!(breakdown.resource_fee, 350); // 100 + 200 + 50
        assert_eq!(breakdown.inclusion_fee, 100); // 450 - 350
        assert_eq!(breakdown.refundable_resource_fee, 200); // total_refundable_resource_fee_charged only
        assert_eq!(breakdown.refundable_fee, 250); // 200 + 50 (includes rent)
        assert_eq!(breakdown.non_refundable_fee, 100);
    }

    // ── auth_entries extraction tests ──────────────────────────────────────────

    /// Helper: build an auth entry XDR base64 for an account (Ed25519) credential.
    fn ed25519_auth_entry_b64(nonce: i64) -> String {
        use stellar_xdr::curr::{
            AccountId, Hash, InvokeContractArgs, PublicKey, ScAddress, ScSymbol, ScVal,
            SorobanAddressCredentials, SorobanAuthorizationEntry, SorobanAuthorizedFunction,
            SorobanAuthorizedInvocation, SorobanCredentials, Uint256,
        };
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
                    [3u8; 32],
                )))),
                nonce,
                signature_expiration_ledger: 100,
                signature: ScVal::Void,
            }),
            root_invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                    contract_address: ScAddress::Contract(Hash([9u8; 32])),
                    function_name: ScSymbol("transfer".try_into().unwrap()),
                    args: vec![].try_into().unwrap(),
                }),
                sub_invocations: vec![].try_into().unwrap(),
            },
        };
        XdrCodec::to_xdr_base64(&entry).expect("encode")
    }

    /// Helper: build an auth entry XDR base64 for a contract (Smart Wallet) credential.
    fn smart_wallet_auth_entry_b64(nonce: i64) -> String {
        use stellar_xdr::curr::{
            Hash, InvokeContractArgs, ScAddress, ScSymbol, ScVal, SorobanAddressCredentials,
            SorobanAuthorizationEntry, SorobanAuthorizedFunction, SorobanAuthorizedInvocation,
            SorobanCredentials,
        };
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: ScAddress::Contract(Hash([5u8; 32])),
                nonce,
                signature_expiration_ledger: 200,
                signature: ScVal::Void,
            }),
            root_invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                    contract_address: ScAddress::Contract(Hash([8u8; 32])),
                    function_name: ScSymbol("invoke".try_into().unwrap()),
                    args: vec![].try_into().unwrap(),
                }),
                sub_invocations: vec![].try_into().unwrap(),
            },
        };
        XdrCodec::to_xdr_base64(&entry).expect("encode")
    }

    #[test]
    fn extract_auth_entries_detects_ed25519() {
        let b64 = ed25519_auth_entry_b64(42);
        let tx_data = serde_json::json!({ "auth": [b64] });
        let entries = extract_auth_entries(&tx_data);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].auth_type, "Ed25519");
        assert!(entries[0].address.starts_with('G'));
        assert!(entries[0].contract_id.is_none());
    }

    #[test]
    fn extract_auth_entries_detects_smart_wallet() {
        let b64 = smart_wallet_auth_entry_b64(99);
        let tx_data = serde_json::json!({ "auth": [b64] });
        let entries = extract_auth_entries(&tx_data);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].auth_type, "Smart Wallet");
        assert!(entries[0].address.starts_with('C'));
        let contract_id = entries[0].contract_id.as_deref().expect("smart wallet must have contract_id");
        assert_eq!(contract_id, entries[0].address);
    }

    #[test]
    fn extract_auth_entries_handles_multiple_entries() {
        let b64_ed = ed25519_auth_entry_b64(1);
        let b64_sw = smart_wallet_auth_entry_b64(2);
        let tx_data = serde_json::json!({ "auth": [b64_ed, b64_sw] });
        let entries = extract_auth_entries(&tx_data);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].auth_type, "Ed25519");
        assert_eq!(entries[1].auth_type, "Smart Wallet");
    }

    #[test]
    fn extract_auth_entries_skips_invalid_payloads() {
        let tx_data = serde_json::json!({ "auth": ["!!!not-valid-xdr!!!"] });
        let entries = extract_auth_entries(&tx_data);
        // Invalid entries are silently skipped; no panic.
        assert!(entries.is_empty());
    }

    #[test]
    fn extract_auth_entries_empty_when_no_auth_field() {
        let tx_data = serde_json::json!({ "hash": "abc123" });
        let entries = extract_auth_entries(&tx_data);
        assert!(entries.is_empty());
    }

    #[test]
    fn existing_ed25519_decoding_unchanged() {
        // Existing auth_signatures behavior must be preserved for Ed25519 entries.
        let b64 = ed25519_auth_entry_b64(7);
        let tx_data = serde_json::json!({ "auth": [b64] });
        // extract_auth_signatures should still return an empty vec
        // because the entry has ScVal::Void (no signature bytes).
        let sigs = extract_auth_signatures(&tx_data);
        assert!(sigs.is_empty(), "no signature bytes in void-signed entry");
    }
}
