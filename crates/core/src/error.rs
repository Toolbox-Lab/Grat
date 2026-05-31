//! Error types for the Prism crate.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standard JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsonRpcError {
    /// Standard JSON-RPC error code.
    pub code: i64,
    /// Human-readable error message.
    pub message: String,
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error (code: {}): {}", self.code, self.message)
    }
}

/// Specific failure kinds for history archive operations.
#[derive(Debug, Error)]
pub enum ArchiveErrorKind {
    /// A fetched archive file did not match its expected checksum.
    #[error("checksum mismatch for '{file}': expected {expected}, got {actual}")]
    ChecksumMismatch {
        file: String,
        expected: String,
        actual: String,
    },

    /// An archive file contained XDR that could not be decoded.
    #[error("malformed XDR in '{file}': {reason}")]
    MalformedXdr { file: String, reason: String },

    /// An archive file could not be fetched from any configured backend.
    #[error("failed to fetch '{file}' from all archive backends: {reason}")]
    FetchFailed { file: String, reason: String },

    /// An archive file could not be decompressed.
    #[error("decompression failed for '{file}': {reason}")]
    DecompressionFailed { file: String, reason: String },
}

/// Top-level error type for all Prism operations.
#[derive(Debug, Error)]
pub enum PrismError {
    /// A network request exceeded the configured timeout duration.
    #[error("RPC request timed out after {timeout_secs}s (method: {method})")]
    NetworkTimeout { method: String, timeout_secs: u64 },

    /// Error communicating with the Soroban RPC endpoint.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Standard JSON-RPC 2.0 error (e.g. Parse error, Invalid request).
    #[error("JSON-RPC error: {0}")]
    JsonRpc(JsonRpcError),

    /// Error fetching or parsing history archive data.
    #[error("Archive error: {0}")]
    ArchiveError(#[from] ArchiveErrorKind),

    /// Error decoding XDR data.
    #[error("XDR error: {0}")]
    XdrError(String),

    /// XDR base64 decoding failed for a specific type.
    ///
    /// Returned by [`crate::xdr::codec::XdrCodec::from_xdr_base64`] when the
    /// input is malformed or does not match the expected XDR type.
    #[error("XDR decoding failed for {type_name}: {reason}")]
    XdrDecodingFailed {
        type_name: &'static str,
        reason: String,
    },

    /// Error parsing WASM or contract spec data.
    #[error("Spec error: {0}")]
    SpecError(String),

    /// Error in the local cache layer.
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Error loading or querying the taxonomy database.
    #[error("Taxonomy error: {0}")]
    TaxonomyError(String),

    /// Error during transaction replay.
    #[error("Replay error: {0}")]
    ReplayError(String),

    /// The requested transaction was not found.
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),

    /// The requested contract was not found on the ledger.
    #[error("Contract not found: {0}")]
    ContractNotFound(String),

    /// An invalid network or configuration was provided.
    #[error("Config error: {0}")]
    ConfigError(String),

    /// An invalid Stellar address was provided.
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Generic internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience Result type for Prism operations.
pub type PrismResult<T> = Result<T, PrismError>;
