//! Parse Soroban authorization entries into a readable auth chain.
//!
//! Auth failures are among the most confusing Soroban errors because the raw
//! `SorobanAuthorizationEntry` XDR hides *who* authorized *what* behind nested
//! credentials and a recursively nested invocation tree. This analyzer flattens
//! that structure: it surfaces the credential (source-account vs. a specific
//! address with its nonce and signature expiry) and walks the
//! `SorobanAuthorizedInvocation` tree into a depth-ordered list of steps, so a
//! reader can see every invocation and where the authorization actually applies.

use crate::error::PrismResult;
use crate::xdr::codec::XdrCodec;
use crate::decode::auth_signature::SignatureKind;
use serde::{Deserialize, Serialize};
use stellar_xdr::curr::{
    AccountId, Hash, PublicKey, ScAddress, ScVal, SorobanAddressCredentials,
    SorobanAuthorizationEntry, SorobanAuthorizedFunction, SorobanAuthorizedInvocation,
    SorobanCredentials, Uint256,
};

/// The credential that authorizes an [`AuthChain`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AuthCredential {
    /// Authorized implicitly by the transaction's source account — no nonce or
    /// signature is carried in the entry itself.
    SourceAccount,
    /// Authorized by a specific address, signed off-chain.
    Address(AddressCredential),
}

/// The address-based credential of an auth entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressCredential {
    /// The authorizing address as a strkey (`G...` account or `C...` contract).
    pub address: String,
    /// Replay-protection nonce chosen by the signer.
    pub nonce: i64,
    /// Ledger sequence at which this signature stops being valid.
    pub signature_expiration_ledger: u32,
    /// Whether a non-void signature payload is present.
    pub signed: bool,
}

impl AddressCredential {
    /// Detect whether this credential is an Ed25519 account or a Smart Wallet
    /// contract by inspecting the address strkey prefix.
    ///
    /// * `G...` → [`SignatureKind::Ed25519`]
    /// * `C...` → [`SignatureKind::SmartWallet`]
    /// * anything else → [`SignatureKind::Unknown`]
    pub fn signature_kind(&self) -> SignatureKind {
        SignatureKind::from_address(&self.address)
    }
}

/// The kind of host function a single invocation step authorizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthFunctionKind {
    /// A contract function call (`ContractFn`).
    ContractFn,
    /// A contract-creation host function (`CreateContractHostFn` and its v2 form).
    CreateContract,
}

/// A single, flattened step in the authorized-invocation tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthInvocation {
    /// Depth in the original tree; the root invocation is `0`.
    pub depth: usize,
    /// What kind of host function this step authorizes.
    pub kind: AuthFunctionKind,
    /// The invoked contract address as a strkey, when known (`ContractFn`).
    pub contract: Option<String>,
    /// The invoked function name, when known (`ContractFn`).
    pub function: Option<String>,
    /// Number of arguments passed to the invocation.
    pub arg_count: usize,
    /// Human-readable argument values decoded from SCVal payloads.
    pub args: Vec<String>,
    /// Compact human-readable description of the authorized target.
    pub target: String,
}

/// A readable authorization chain parsed from a raw auth entry: the credential
/// state plus every invocation it covers, in depth-first order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthChain {
    /// Who authorized this chain and how.
    pub credential: AuthCredential,
    /// Every authorized invocation, flattened depth-first from the root.
    pub invocations: Vec<AuthInvocation>,
}

impl AuthChain {
    /// Parse an [`AuthChain`] from a base64-encoded `SorobanAuthorizationEntry`.
    pub fn from_xdr_base64(b64: &str) -> PrismResult<Self> {
        let entry = SorobanAuthorizationEntry::from_xdr_base64(b64)?;
        Ok(Self::from_entry(&entry))
    }

    /// Parse an [`AuthChain`] from a decoded `SorobanAuthorizationEntry`.
    pub fn from_entry(entry: &SorobanAuthorizationEntry) -> Self {
        let credential = parse_credential(&entry.credentials);
        let mut invocations = Vec::new();
        walk_invocation(&entry.root_invocation, 0, &mut invocations);
        Self {
            credential,
            invocations,
        }
    }
}

/// Extract the credential state (address + nonce, or source account) from the
/// entry's `SorobanCredentials`.
fn parse_credential(credentials: &SorobanCredentials) -> AuthCredential {
    match credentials {
        SorobanCredentials::SourceAccount => AuthCredential::SourceAccount,
        SorobanCredentials::Address(addr) => {
            AuthCredential::Address(parse_address_credential(addr))
        }
    }
}

fn parse_address_credential(creds: &SorobanAddressCredentials) -> AddressCredential {
    AddressCredential {
        address: scaddress_to_strkey(&creds.address),
        nonce: creds.nonce,
        signature_expiration_ledger: creds.signature_expiration_ledger,
        signed: creds.signature != ScVal::Void,
    }
}

/// Recursively flatten an `AuthorizedInvocation` tree into depth-ordered steps.
fn walk_invocation(
    invocation: &SorobanAuthorizedInvocation,
    depth: usize,
    out: &mut Vec<AuthInvocation>,
) {
    out.push(parse_function(&invocation.function, depth));
    for sub in invocation.sub_invocations.iter() {
        walk_invocation(sub, depth + 1, out);
    }
}

fn parse_function(function: &SorobanAuthorizedFunction, depth: usize) -> AuthInvocation {
    match function {
        SorobanAuthorizedFunction::ContractFn(args) => {
            let contract = scaddress_to_strkey(&args.contract_address);
            let function = args.function_name.to_string();
            let readable_args: Vec<String> =
                args.args.iter().map(scval_to_readable_string).collect();
            let target = format!("{}.{}({})", contract, function, readable_args.join(", "));

            AuthInvocation {
                depth,
                kind: AuthFunctionKind::ContractFn,
                contract: Some(contract),
                function: Some(function),
                arg_count: readable_args.len(),
                args: readable_args,
                target,
            }
        }
        // Both `CreateContractHostFn` and the v2 form authorize a contract
        // creation; the contract address is not yet known at this point.
        _ => AuthInvocation {
            depth,
            kind: AuthFunctionKind::CreateContract,
            contract: None,
            function: None,
            arg_count: 0,
            args: Vec::new(),
            target: "create_contract".to_string(),
        },
    }
}

/// Render an `ScVal` argument in a compact form suitable for auth summaries.
pub fn scval_to_readable_string(val: &ScVal) -> String {
    match val {
        ScVal::Void => "void".to_string(),
        ScVal::Bool(b) => b.to_string(),
        ScVal::Symbol(sym) => sym.to_string(),
        ScVal::String(s) => format!("\"{}\"", s),
        ScVal::U32(u) => u.to_string(),
        ScVal::I32(i) => i.to_string(),
        ScVal::U64(u) => u.to_string(),
        ScVal::I64(i) => i.to_string(),
        ScVal::Timepoint(t) => t.to_string(),
        ScVal::Duration(d) => d.to_string(),
        ScVal::U128(u) => (((u.hi as u128) << 64) | (u.lo as u128)).to_string(),
        ScVal::I128(i) => (((i.hi as i128) << 64) | (i.lo as u128 as i128)).to_string(),
        ScVal::Address(address) => scaddress_to_strkey(address),
        ScVal::Bytes(bytes) => format!(
            "0x{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ),
        ScVal::Vec(Some(vec)) => {
            let items: Vec<String> = vec.iter().map(scval_to_readable_string).collect();
            format!("[{}]", items.join(", "))
        }
        ScVal::Vec(None) => "[]".to_string(),
        ScVal::Map(Some(map)) => {
            let items: Vec<String> = map
                .iter()
                .map(|entry| {
                    format!(
                        "{}: {}",
                        scval_to_readable_string(&entry.key),
                        scval_to_readable_string(&entry.val)
                    )
                })
                .collect();
            format!("{{{}}}", items.join(", "))
        }
        ScVal::Map(None) => "{}".to_string(),
        ScVal::Error(err) => format!("error:{err:?}"),
        other => format!("{other:?}"),
    }
}

/// Render an `ScAddress` as a strkey (`G...` for accounts, `C...` for contracts).
pub(crate) fn scaddress_to_strkey(address: &ScAddress) -> String {
    match address {
        ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(bytes)))) => {
            stellar_strkey::ed25519::PublicKey(*bytes).to_string()
        }
        ScAddress::Contract(Hash(bytes)) => stellar_strkey::Contract(*bytes).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::curr::{InvokeContractArgs, ScSymbol};

    fn account_address(seed: u8) -> ScAddress {
        ScAddress::Account(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(
            [seed; 32],
        ))))
    }

    fn contract_address(seed: u8) -> ScAddress {
        ScAddress::Contract(Hash([seed; 32]))
    }

    fn contract_fn(addr: ScAddress, name: &str, args: Vec<ScVal>) -> SorobanAuthorizedFunction {
        SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
            contract_address: addr,
            function_name: ScSymbol(name.try_into().unwrap()),
            args: args.try_into().unwrap(),
        })
    }

    fn invocation(
        function: SorobanAuthorizedFunction,
        subs: Vec<SorobanAuthorizedInvocation>,
    ) -> SorobanAuthorizedInvocation {
        SorobanAuthorizedInvocation {
            function,
            sub_invocations: subs.try_into().unwrap(),
        }
    }

    fn empty_subs() -> Vec<SorobanAuthorizedInvocation> {
        Vec::new()
    }

    #[test]
    fn source_account_credential_is_recognized() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: invocation(
                contract_fn(contract_address(7), "transfer", vec![ScVal::U32(1)]),
                empty_subs(),
            ),
        };

        let chain = AuthChain::from_entry(&entry);
        assert_eq!(chain.credential, AuthCredential::SourceAccount);
        assert_eq!(chain.invocations.len(), 1);
        let root = &chain.invocations[0];
        assert_eq!(root.depth, 0);
        assert_eq!(root.kind, AuthFunctionKind::ContractFn);
        assert_eq!(root.function.as_deref(), Some("transfer"));
        assert_eq!(root.arg_count, 1);
        assert_eq!(root.args, vec!["1"]);
        assert!(root.target.contains(".transfer(1)"));
        assert!(root.contract.as_deref().unwrap().starts_with('C'));
    }

    #[test]
    fn address_credential_extracts_nonce_and_signed_state() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: account_address(3),
                nonce: 42,
                signature_expiration_ledger: 1000,
                signature: ScVal::Bool(true),
            }),
            root_invocation: invocation(
                contract_fn(contract_address(9), "approve", vec![]),
                empty_subs(),
            ),
        };

        let chain = AuthChain::from_entry(&entry);
        match chain.credential {
            AuthCredential::Address(creds) => {
                assert!(creds.address.starts_with('G'));
                assert_eq!(creds.nonce, 42);
                assert_eq!(creds.signature_expiration_ledger, 1000);
                assert!(creds.signed);
            }
            other => panic!("expected address credential, got {other:?}"),
        }
    }

    #[test]
    fn void_signature_is_reported_as_unsigned() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: account_address(1),
                nonce: 0,
                signature_expiration_ledger: 0,
                signature: ScVal::Void,
            }),
            root_invocation: invocation(
                contract_fn(contract_address(2), "f", vec![]),
                empty_subs(),
            ),
        };

        let chain = AuthChain::from_entry(&entry);
        match chain.credential {
            AuthCredential::Address(creds) => assert!(!creds.signed),
            other => panic!("expected address credential, got {other:?}"),
        }
    }

    #[test]
    fn nested_invocations_are_flattened_depth_first() {
        // root -> [child_a -> [grandchild], child_b]
        let grandchild = invocation(
            contract_fn(contract_address(30), "gc", vec![]),
            empty_subs(),
        );
        let child_a = invocation(
            contract_fn(
                contract_address(20),
                "a",
                vec![ScVal::U32(1), ScVal::U32(2)],
            ),
            vec![grandchild],
        );
        let child_b = invocation(contract_fn(contract_address(21), "b", vec![]), empty_subs());
        let root = invocation(
            contract_fn(contract_address(10), "root", vec![]),
            vec![child_a, child_b],
        );

        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: root,
        };

        let chain = AuthChain::from_entry(&entry);
        let steps: Vec<(usize, &str)> = chain
            .invocations
            .iter()
            .map(|i| (i.depth, i.function.as_deref().unwrap()))
            .collect();

        assert_eq!(steps, vec![(0, "root"), (1, "a"), (2, "gc"), (1, "b")]);
        // Arg counts and readable argument values are preserved per step.
        assert_eq!(chain.invocations[1].arg_count, 2);
        assert_eq!(chain.invocations[1].args, vec!["1", "2"]);
        assert!(chain.invocations[1].target.contains(".a(1, 2)"));
    }

    #[test]
    fn contract_invocation_decodes_readable_argument_values() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: invocation(
                contract_fn(
                    contract_address(8),
                    "transfer",
                    vec![
                        ScVal::Address(account_address(4)),
                        ScVal::Bool(true),
                        ScVal::Vec(Some(
                            vec![ScVal::U32(10), ScVal::I32(-2)].try_into().unwrap(),
                        )),
                    ],
                ),
                empty_subs(),
            ),
        };

        let chain = AuthChain::from_entry(&entry);
        let root = &chain.invocations[0];

        assert_eq!(root.function.as_deref(), Some("transfer"));
        assert_eq!(root.arg_count, 3);
        assert!(root.args[0].starts_with('G'));
        assert_eq!(root.args[1], "true");
        assert_eq!(root.args[2], "[10, -2]");
        assert!(root.target.contains(".transfer("));
        assert!(root.target.contains("true"));
        assert!(root.target.contains("[10, -2]"));
    }

    #[test]
    fn round_trips_through_base64() {
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: invocation(
                contract_fn(contract_address(5), "mint", vec![ScVal::U32(7)]),
                empty_subs(),
            ),
        };
        let b64 = XdrCodec::to_xdr_base64(&entry).expect("encode");

        let chain = AuthChain::from_xdr_base64(&b64).expect("parse");
        assert_eq!(chain.credential, AuthCredential::SourceAccount);
        assert_eq!(chain.invocations.len(), 1);
        assert_eq!(chain.invocations[0].function.as_deref(), Some("mint"));
    }

    #[test]
    fn invalid_base64_is_an_error() {
        assert!(AuthChain::from_xdr_base64("!!!not-valid!!!").is_err());
    }

    // --- Issue #271: signature_kind() on AddressCredential ---

    #[test]
    fn address_credential_ed25519_kind_from_account_address() {
        use crate::decode::auth_signature::SignatureKind;
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: account_address(1),
                nonce: 0,
                signature_expiration_ledger: 0,
                signature: ScVal::Void,
            }),
            root_invocation: invocation(
                contract_fn(contract_address(2), "fn", vec![]),
                empty_subs(),
            ),
        };
        let chain = AuthChain::from_entry(&entry);
        match chain.credential {
            AuthCredential::Address(cred) => {
                assert_eq!(cred.signature_kind(), SignatureKind::Ed25519);
                assert!(cred.address.starts_with('G'));
            }
            other => panic!("expected Address credential, got {other:?}"),
        }
    }

    #[test]
    fn address_credential_smart_wallet_kind_from_contract_address() {
        use crate::decode::auth_signature::SignatureKind;
        let entry = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: contract_address(5),
                nonce: 0,
                signature_expiration_ledger: 0,
                signature: ScVal::Void,
            }),
            root_invocation: invocation(
                contract_fn(contract_address(6), "fn", vec![]),
                empty_subs(),
            ),
        };
        let chain = AuthChain::from_entry(&entry);
        match chain.credential {
            AuthCredential::Address(cred) => {
                assert_eq!(cred.signature_kind(), SignatureKind::SmartWallet);
                assert!(cred.address.starts_with('C'));
            }
            other => panic!("expected Address credential, got {other:?}"),
        }
    }
}
