pub mod auth;
pub mod auth_address_nonce;
pub mod auth_signature;
pub mod context;
pub mod contract_error;
pub mod contract_error_resolver;
pub mod cross_contract;
pub mod decode_context;
pub mod diagnostic;
pub mod fee_analyzer;
pub mod event_walker;
pub mod host_error;
pub mod mappings;
pub mod report;
pub mod scval_to_json;
pub mod walker;

pub use auth::{
    AddressCredential, AuthChain, AuthCredential, AuthFunctionKind, AuthInvocation,
    AuthorizationType,
};
pub use auth_address_nonce::AddressWithNonce;
pub use scval_to_json::scval_to_json;
pub use walker::{
    walk_diagnostic_events, DiagnosticEventKind, DiagnosticEventWalker, StructuredDiagnosticEvent,
};

use crate::decode::fee_analyzer::inject_fee_metadata;
use crate::error::{GratError, GratResult};
use crate