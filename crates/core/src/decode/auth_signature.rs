use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use stellar_xdr::curr::{
    Limits, ReadXdr, ScMap, ScVal, SorobanAddressCredentials, SorobanAuthorizationEntry,
    SorobanCredentials,
};

/// The kind of signer for a Soroban authorization credential.
///
/// An address starting with `G` is a classic Ed25519 key-pair account.
/// An address starting with `C` is a smart contract (smart wallet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureKind {
    /// Classic Ed25519 public-key account (`G...` strkey).
    Ed25519,
    /// Smart-wallet contract account (`C...` strkey).
    SmartWallet,
    /// Address format not recognised (should not occur in practice).
    Unknown,
}

impl SignatureKind {
    /// Infer the kind from a strkey address string.
    ///
    /// * Addresses that start with `'G'` are Ed25519 accounts.
    /// * Addresses that start with `'C'` are smart-wallet contracts.
    /// * Everything else is `Unknown`.
    pub fn from_address(address: &str) -> Self {
        match address.chars().next() {
            Some('G') => SignatureKind::Ed25519,
            Some('C') => SignatureKind::SmartWallet,
            _ => SignatureKind::Unknown,
        }
    }

    /// Human-readable label for display purposes.
    pub fn label(self) -> &'static str {
        match self {
            SignatureKind::Ed25519 => "Ed25519",
            SignatureKind::SmartWallet => "Smart Wallet",
            SignatureKind::Unknown => "Unknown",
        }
    }
}

/// Structured information about a single authorization credential, combining
/// the detected [`SignatureKind`] with the address (public key or contract ID)
/// and any decoded signature hex strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSignatureInfo {
    /// Whether this is an Ed25519 account or a smart-wallet contract.
    pub kind: SignatureKind,
    /// The authorizing address as a strkey.
    ///
    /// * For `Ed25519` this is the public key (`G...`).
    /// * For `SmartWallet` this is the contract ID (`C...`).
    pub address: String,
    /// Decoded signature hex strings extracted from the credential payload.
    /// Empty when no signatures are present (e.g. `SourceAccount` credentials).
    pub signatures: Vec<String>,
}

impl AuthSignatureInfo {
    /// Build an [`AuthSignatureInfo`] from a `SorobanAddressCredentials` value.
    pub fn from_address_credentials(creds: &SorobanAddressCredentials) -> Self {
        use crate::decode::auth::scaddress_to_strkey;
        let address = scaddress_to_strkey(&creds.address);
        let kind = SignatureKind::from_address(&address);
        let signatures = extract_signatures_from_scval(&creds.signature);
        Self {
            kind,
            address,
            signatures,
        }
    }
}

/// Decode a single raw signature bytes value into a hex string.
/// Returns an error label string if bytes are empty or not a valid 64-byte ed25519 signature.
pub fn decode_signature_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "<invalid: empty signature>".to_string();
    }
    if bytes.len() != 64 {
        return format!("<invalid: expected 64 bytes, got {}>", bytes.len());
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Extract signature hex strings from a `SorobanAuthorizationEntry` XDR base64 string.
/// For each signature found in the entry's credentials, decodes raw bytes to hex.
/// Returns a list of hex strings (or error labels for malformed signatures).
pub fn decode_auth_entry_signatures(auth_entry_b64: &str) -> Vec<String> {
    let bytes = match STANDARD.decode(auth_entry_b64) {
        Ok(b) => b,
        Err(_) => return vec!["<invalid: base64 decode failed>".to_string()],
    };

    let entry = match SorobanAuthorizationEntry::from_xdr(&bytes, Limits::none()) {
        Ok(e) => e,
        Err(_) => return vec!["<invalid: xdr decode failed>".to_string()],
    };

    match entry.credentials {
        SorobanCredentials::SourceAccount => vec![],
        SorobanCredentials::Address(SorobanAddressCredentials { signature, .. }) => {
            extract_signatures_from_scval(&signature)
        }
    }
}

/// Decode a `SorobanAuthorizationEntry` XDR base64 string into an [`AuthSignatureInfo`],
/// detecting whether the credential is Ed25519 or Smart Wallet.
///
/// Returns `None` for source-account credentials (which carry no address).
/// Returns `Err` for base64 / XDR decode failures.
pub fn decode_auth_entry_signature_info(
    auth_entry_b64: &str,
) -> Result<Option<AuthSignatureInfo>, String> {
    let bytes = match STANDARD.decode(auth_entry_b64) {
        Ok(b) => b,
        Err(_) => return Err("<invalid: base64 decode failed>".to_string()),
    };

    let entry = match SorobanAuthorizationEntry::from_xdr(&bytes, Limits::none()) {
        Ok(e) => e,
        Err(_) => return Err("<invalid: xdr decode failed>".to_string()),
    };

    match &entry.credentials {
        SorobanCredentials::SourceAccount => Ok(None),
        SorobanCredentials::Address(creds) => {
            Ok(Some(AuthSignatureInfo::from_address_credentials(creds)))
        }
    }
}

/// Recursively extract signature hex strings from a ScVal.
/// Handles both a single ScBytes (direct signature) and a ScMap / ScVec of signatures.
fn extract_signatures_from_scval(val: &ScVal) -> Vec<String> {
    match val {
        // Direct bytes — treat as a raw signature
        ScVal::Bytes(sc_bytes) => vec![decode_signature_bytes(sc_bytes.as_ref())],

        // Map entries: standard ed25519 account signature has { public_key: bytes, signature: bytes }
        // Only extract from "signature" keyed entries to avoid decoding public_key bytes.
        ScVal::Map(Some(ScMap(entries))) => {
            let mut results = Vec::new();
            for entry in entries.iter() {
                let key_str = match &entry.key {
                    ScVal::Symbol(s) => s.to_string(),
                    ScVal::String(s) => s.to_string(),
                    _ => continue,
                };
                if key_str == "signature" {
                    results.extend(extract_signatures_from_scval(&entry.val));
                }
            }
            results
        }

        // Vec of signature entries (e.g., multiple account signatures)
        ScVal::Vec(Some(vec)) => vec
            .iter()
            .flat_map(|v| extract_signatures_from_scval(v))
            .collect(),

        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{
        AccountId, Hash, InvokeContractArgs, PublicKey, ScAddress, ScSymbol,
        SorobanAuthorizedFunction, SorobanAuthorizedInvocation, Uint256,
    };

    fn account_address(seed: u8) -> ScAddress {
        ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([seed; 32]))))
    }

    fn contract_address(seed: u8) -> ScAddress {
        ScAddress::Contract(Hash([seed; 32]))
    }

    fn make_auth_entry(
        address: ScAddress,
        signature: ScVal,
    ) -> SorobanAuthorizationEntry {
        SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address,
                nonce: 0,
                signature_expiration_ledger: 0,
                signature,
            }),
            root_invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                    contract_address: contract_address(1),
                    function_name: ScSymbol("f".try_into().unwrap()),
                    args: vec![].try_into().unwrap(),
                }),
                sub_invocations: vec![].try_into().unwrap(),
            },
        }
    }

    #[test]
    fn decode_valid_64_bytes() {
        let bytes = vec![0xabu8; 64];
        let hex = decode_signature_bytes(&bytes);
        assert_eq!(hex.len(), 128);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(hex.starts_with("ab"));
    }

    #[test]
    fn decode_empty_bytes_returns_error_label() {
        let result = decode_signature_bytes(&[]);
        assert_eq!(result, "<invalid: empty signature>");
    }

    #[test]
    fn decode_wrong_length_returns_error_label() {
        let result = decode_signature_bytes(&[0u8; 32]);
        assert!(result.starts_with("<invalid: expected 64 bytes"));
    }

    #[test]
    fn decode_invalid_base64_returns_error_label() {
        let result = decode_auth_entry_signatures("!!!not-base64!!!");
        assert_eq!(result, vec!["<invalid: base64 decode failed>"]);
    }

    #[test]
    fn decode_invalid_xdr_returns_error_label() {
        // Valid base64 but not a valid SorobanAuthorizationEntry
        let result = decode_auth_entry_signatures("AAAA");
        assert_eq!(result, vec!["<invalid: xdr decode failed>"]);
    }

    // --- Issue #271: SignatureKind detection tests ---

    #[test]
    fn signature_kind_from_g_address_is_ed25519() {
        let kind = SignatureKind::from_address("GABC...");
        assert_eq!(kind, SignatureKind::Ed25519);
        assert_eq!(kind.label(), "Ed25519");
    }

    #[test]
    fn signature_kind_from_c_address_is_smart_wallet() {
        let kind = SignatureKind::from_address("CABC...");
        assert_eq!(kind, SignatureKind::SmartWallet);
        assert_eq!(kind.label(), "Smart Wallet");
    }

    #[test]
    fn signature_kind_from_unknown_prefix_is_unknown() {
        let kind = SignatureKind::from_address("XABC...");
        assert_eq!(kind, SignatureKind::Unknown);
        assert_eq!(kind.label(), "Unknown");
    }

    #[test]
    fn signature_kind_from_empty_address_is_unknown() {
        let kind = SignatureKind::from_address("");
        assert_eq!(kind, SignatureKind::Unknown);
    }

    #[test]
    fn auth_signature_info_ed25519_from_account_address() {
        use crate::xdr::codec::XdrCodec;
        let entry = make_auth_entry(account_address(3), ScVal::Void);
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");
        let info = decode_auth_entry_signature_info(&b64)
            .expect("no error")
            .expect("address credential");
        assert_eq!(info.kind, SignatureKind::Ed25519);
        assert!(info.address.starts_with('G'), "address should start with G, got {}", info.address);
    }

    #[test]
    fn auth_signature_info_smart_wallet_from_contract_address() {
        use crate::xdr::codec::XdrCodec;
        let entry = make_auth_entry(contract_address(7), ScVal::Void);
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");
        let info = decode_auth_entry_signature_info(&b64)
            .expect("no error")
            .expect("address credential");
        assert_eq!(info.kind, SignatureKind::SmartWallet);
        assert!(info.address.starts_with('C'), "address should start with C, got {}", info.address);
    }

    #[test]
    fn contract_id_extracted_for_smart_wallet() {
        use crate::xdr::codec::XdrCodec;
        let entry = make_auth_entry(contract_address(42), ScVal::Void);
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");
        let info = decode_auth_entry_signature_info(&b64)
            .expect("no error")
            .expect("address credential");
        // Contract ID must be a valid strkey starting with 'C'.
        assert!(info.address.starts_with('C'));
        assert!(!info.address.is_empty());
    }

    #[test]
    fn decode_auth_entry_signature_info_invalid_payload() {
        let result = decode_auth_entry_signature_info("!!!bad-base64!!!");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "<invalid: base64 decode failed>");
    }

    #[test]
    fn decode_auth_entry_signature_info_source_account_returns_none() {
        use crate::xdr::codec::XdrCodec;
        use stellar_xdr::curr::InvokeContractArgs;
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                    contract_address: contract_address(1),
                    function_name: ScSymbol("g".try_into().unwrap()),
                    args: vec![].try_into().unwrap(),
                }),
                sub_invocations: vec![].try_into().unwrap(),
            },
        };
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");
        let result = decode_auth_entry_signature_info(&b64).expect("no error");
        assert!(result.is_none(), "SourceAccount should yield None");
    }

    #[test]
    fn existing_decode_auth_entry_signatures_unchanged() {
        // Existing behavior: invalid base64 returns a single error label.
        let result = decode_auth_entry_signatures("not-valid-base64!!!");
        assert_eq!(result, vec!["<invalid: base64 decode failed>"]);

        // Existing behavior: invalid XDR returns a single error label.
        let result = decode_auth_entry_signatures("AAAA");
        assert_eq!(result, vec!["<invalid: xdr decode failed>"]);
    }
}
