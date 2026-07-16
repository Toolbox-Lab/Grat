use base64::{engine::general_purpose::STANDARD, Engine as _};
use stellar_xdr::curr::{
    Limits, ReadXdr, ScMap, ScVal, SorobanAddressCredentials, SorobanAuthorizationEntry,
    SorobanCredentials,
};

///
///
pub fn decode_signature_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "<invalid: empty signature>".to_string();
    }
    if bytes.len() != 64 {
        return format!("<invalid: expected 64 bytes, got {}>", bytes.len());
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

///
///
///
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

///
///
fn extract_signatures_from_scval(val: &ScVal) -> Vec<String> {
    match val {
        ScVal::Bytes(sc_bytes) => vec![decode_signature_bytes(sc_bytes.as_ref())],

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
        let result = decode_auth_entry_signatures("AAAA");
        assert_eq!(result, vec!["<invalid: xdr decode failed>"]);
    }
}
