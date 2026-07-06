

use serde::{Deserialize, Serialize};

/// A summary of a decoded authorization entry, including its detected type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthEntryInfo {
    /// Human-readable label: "Ed25519" or "Smart Wallet".
    pub auth_type: String,
    /// The authorizing address as a strkey (`G...` for Ed25519, `C...` for Smart Wallet).
    pub address: String,
    /// For Smart Wallet entries: the contract ID in strkey form (`C...`).
    /// `None` for Ed25519 entries.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub contract_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCause {

    pub description: String,

    pub likelihood: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedFix {

    pub description: String,

    pub difficulty: String,

    pub requires_upgrade: bool,

    pub example: Option<String>,

    pub id: String,

    pub remedy_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractErrorInfo {

    pub contract_id: String,

    pub error_code: u32,

    pub error_name: Option<String>,

    pub doc_comment: Option<String>,
    
    pub learn_more: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContext {

    pub tx_hash: String,

    pub ledger_sequence: u32,

    pub function_name: Option<String>,

    pub arguments: Vec<String>,

    pub return_value: Option<String>,

    pub fee: FeeBreakdown,

    pub resources: ResourceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeBreakdown {
    pub total_charged_fee: i64,
    pub inclusion_fee: i64,
    /// Total resource fee: non_refundable + refundable_resource_fee + rent_fee_charged.
    pub resource_fee: i64,
    /// The refundable portion of the resource fee charged
    /// (`total_refundable_resource_fee_charged` from `SorobanTransactionMetaExtV1`).
    /// This amount may be partially returned to the submitter if resources are unused.
    pub refundable_resource_fee: i64,
    /// `total_refundable_resource_fee_charged + rent_fee_charged`.
    /// Kept for backwards compatibility with existing callers.
    pub refundable_fee: i64,
    pub non_refundable_fee: i64,
    pub bid_fee: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub cpu_instructions_used: u64,
    pub cpu_instructions_limit: u64,
    pub memory_bytes_used: u64,
    pub memory_bytes_limit: u64,
    pub read_bytes: u64,
    pub read_bytes_limit: u64,
    pub write_bytes: u64,
}

/// Pinpoints the exact contract and function where a cross-contract call chain failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAttribution {
    /// The contract address that directly caused the failure.
    pub contract_address: String,
    /// The function name at the point of failure, if determinable.
    pub function_name: Option<String>,
    /// The call depth at which the failure occurred (0 = top-level invoker).
    pub call_depth: usize,
    /// Human-readable description of where in the call chain the failure originated.
    pub origin_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {

    pub error_category: String,

    pub error_code: u32,

    pub error_name: String,

    pub summary: String,

    pub detailed_explanation: String,

    pub severity: Severity,

    pub root_causes: Vec<RootCause>,

    pub suggested_fixes: Vec<SuggestedFix>,

    pub contract_error: Option<ContractErrorInfo>,

    pub transaction_context: Option<TransactionContext>,

    pub related_errors: Vec<String>,

    /// Present when a cross-contract call chain was detected and the failure
    /// was attributed to a specific sub-contract, not the top-level invoker.
    pub cross_contract_attribution: Option<FailureAttribution>,

    /// Decoded hex strings for ed25519 signatures found in auth entries.
    /// Malformed or empty byte sequences produce a human-readable error label.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub auth_signatures: Vec<String>,

    /// Typed summaries of each authorization entry found in the transaction,
    /// labelled as Ed25519 or Smart Wallet with the relevant address/contract_id.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub auth_entries: Vec<AuthEntryInfo>,

    /// The contract address most likely responsible for the failure, extracted
    /// from diagnostic event traces.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub failing_contract_id: Option<String>,

    pub learn_more: String,
}

impl DiagnosticReport {

    pub fn new(category: &str, code: u32, name: &str, summary: &str) -> Self {
        Self {
            error_category: category.to_string(),
            error_code: code,
            error_name: name.to_string(),
            summary: summary.to_string(),
            detailed_explanation: String::new(),
            severity: Severity::Error,
            root_causes: Vec::new(),
            suggested_fixes: Vec::new(),
            contract_error: None,
            transaction_context: None,
            related_errors: Vec::new(),
            cross_contract_attribution: None,
            auth_signatures: Vec::new(),
            auth_entries: Vec::new(),
            failing_contract_id: None,
            learn_more: "https://developers.stellar.org/docs/learn/smart-contracts/errors".to_string(),  
       }
    }
}
