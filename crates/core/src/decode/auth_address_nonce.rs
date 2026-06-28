//! Decode the address-with-nonce credential from a Soroban authorization entry.
//!
//! Soroban auth entries carry a nonce that is bound to an authorizing address to
//! prevent signature replay. In the raw XDR this pair lives inside
//! `SorobanAddressCredentials` (historically named `SorobanAddressWithNonce`),
//! nested under the entry's credentials and hard to read at a glance. This module
//! pulls that pair out and formats it so a reader can clearly see *who* authorized
//! the entry and the *specific nonce* that protects it.

use serde::{Deserialize, Serialize};
use std::fmt;
use stellar_xdr::curr::{SorobanAddressCredentials, SorobanAuthorizationEntry, SorobanCredentials};

use crate::decode::auth::scaddress_to_strkey;
use crate::error::PrismResult;
use crate::xdr::codec::XdrCodec;

/// An authorizing address paired with its replay-protection nonce, extracted from
/// a `SorobanAddressCredentials` (a.k.a. `SorobanAddressWithNonce`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressWithNonce {
    /// The authorizing address as a strkey (`G...` account or `C...` contract).
    pub address: String,
    /// The replay-protection nonce bound to this address.
    pub nonce: i64,
}

impl AddressWithNonce {
    /// Decode the address-with-nonce from a base64 `SorobanAuthorizationEntry`.
    ///
    /// Returns `Ok(None)` for source-account credentials, which carry no address
    /// or nonce of their own — they reuse the transaction's source account.
    pub fn from_auth_entry_base64(b64: &str) -> PrismResult<Option<Self>> {
        let entry = SorobanAuthorizationEntry::from_xdr_base64(b64)?;
        Ok(Self::from_entry(&entry))
    }

    /// Extract the address-with-nonce from a decoded auth entry, when it uses
    /// address-based credentials.
    pub fn from_entry(entry: &SorobanAuthorizationEntry) -> Option<Self> {
        match &entry.credentials {
            SorobanCredentials::SourceAccount => None,
            SorobanCredentials::Address(creds) => Some(Self::from_credentials(creds)),
        }
    }

    fn from_credentials(creds: &SorobanAddressCredentials) -> Self {
        Self {
            address: scaddress_to_strkey(&creds.address),
            nonce: creds.nonce,
        }
    }
}

impl fmt::Display for AddressWithNonce {
    /// Render the address and its nonce on separate, clearly labeled lines.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Address: {}", self.address)?;
        write!(f, "Nonce:   {}", self.nonce)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{
        AccountId, Hash, InvokeContractArgs, PublicKey, ScAddress, ScSymbol, ScVal,
        SorobanAuthorizedFunction, SorobanAuthorizedInvocation, Uint256,
    };

    fn account_address(seed: u8) -> ScAddress {
        ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([seed; 32]))))
    }

    fn contract_address(seed: u8) -> ScAddress {
        ScAddress::Contract(Hash([seed; 32]))
    }

    fn root_invocation(addr: ScAddress) -> SorobanAuthorizedInvocation {
        SorobanAuthorizedInvocation {
            function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                contract_address: addr,
                function_name: ScSymbol("f".try_into().unwrap()),
                args: Vec::new().try_into().unwrap(),
            }),
            sub_invocations: Vec::new().try_into().unwrap(),
        }
    }

    fn address_entry(addr: ScAddress, nonce: i64) -> SorobanAuthorizationEntry {
        SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: addr.clone(),
                nonce,
                signature_expiration_ledger: 0,
                signature: ScVal::Void,
            }),
            root_invocation: root_invocation(addr),
        }
    }

    #[test]
    fn extracts_account_address_and_nonce() {
        let entry = address_entry(account_address(3), 42);
        let parsed = AddressWithNonce::from_entry(&entry).expect("address credential");
        assert!(parsed.address.starts_with('G'));
        assert_eq!(parsed.nonce, 42);
    }

    #[test]
    fn extracts_contract_address_and_nonce() {
        let entry = address_entry(contract_address(9), -7);
        let parsed = AddressWithNonce::from_entry(&entry).expect("address credential");
        assert!(parsed.address.starts_with('C'));
        assert_eq!(parsed.nonce, -7);
    }

    #[test]
    fn source_account_has_no_address_with_nonce() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: root_invocation(contract_address(1)),
        };
        assert!(AddressWithNonce::from_entry(&entry).is_none());
    }

    #[test]
    fn display_separates_address_and_nonce() {
        let entry = address_entry(account_address(5), 1234);
        let parsed = AddressWithNonce::from_entry(&entry).expect("address credential");
        let rendered = parsed.to_string();
        assert!(rendered.contains(&format!("Address: {}", parsed.address)));
        assert!(rendered.contains("Nonce:   1234"));
        // Address and nonce land on separate lines.
        assert_eq!(rendered.lines().count(), 2);
    }

    #[test]
    fn round_trips_through_base64() {
        let entry = address_entry(account_address(7), 99);
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");
        let parsed = AddressWithNonce::from_auth_entry_base64(&b64)
            .expect("decode")
            .expect("address credential");
        assert!(parsed.address.starts_with('G'));
        assert_eq!(parsed.nonce, 99);
    }

    #[test]
    fn source_account_base64_yields_none() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: root_invocation(contract_address(2)),
        };
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");
        let parsed = AddressWithNonce::from_auth_entry_base64(&b64).expect("decode");
        assert!(parsed.is_none());
    }

    #[test]
    fn invalid_base64_is_an_error() {
        assert!(AddressWithNonce::from_auth_entry_base64("!!!not-valid!!!").is_err());
    }
}
