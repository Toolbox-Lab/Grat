//! Tier 1: Error Decode Engine.
//!
//! Provides error classification, contract error resolution, diagnostic event
//! analysis, context enrichment, and report generation.

pub mod context;
pub mod contract_error;
pub mod diagnostic;
pub mod host_error;
pub mod mappings;
pub mod report;

use crate::error::PrismResult;
use crate::types::report::DiagnosticReport;

/// Filter transaction data to focus on a specific operation index.
fn filter_transaction_by_operation(
    tx_data: &mut serde_json::Value,
    op_index: usize,
) -> PrismResult<()> {
    if let Some(events) = tx_data.get_mut("events") {
        if let Some(contract_events) = events.get_mut("contractEventsXdr") {
            if let Some(events_array) = contract_events.as_array_mut() {
                if op_index < events_array.len() {
                    let target_events = events_array[op_index].clone();
                    *events_array = vec![target_events];
                } else {
                    *events_array = vec![];
                }
            }
        }
    }

    if let Some(diagnostic_events) = tx_data.get_mut("diagnosticEventsXdr") {
        if let Some(events_array) = diagnostic_events.as_array_mut() {
            if op_index == 0 && !events_array.is_empty() {
                let first_event = events_array[0].clone();
                *events_array = vec![first_event];
            } else {
                *events_array = vec![];
            }
        }
    }

    Ok(())
}

/// Decode a transaction error from its hash, returning a full diagnostic report.
///
/// This is the main entry point for Tier 1 functionality.
pub async fn decode_transaction(
    tx_hash: &str,
    network: &crate::types::config::NetworkConfig,
) -> PrismResult<DiagnosticReport> {
    decode_transaction_with_op_filter(tx_hash, network, None).await
}

/// Decode a transaction error from its hash, returning a full diagnostic report.
/// Optionally filter to focus on a specific operation index.
pub async fn decode_transaction_with_op_filter(
    tx_hash: &str,
    network: &crate::types::config::NetworkConfig,
    op_index: Option<usize>,
) -> PrismResult<DiagnosticReport> {
    let rpc = crate::rpc::SorobanRpcClient::new(network);
    let tx_data = rpc.get_transaction(tx_hash).await?;
    let mut tx_data = serde_json::to_value(tx_data)
        .map_err(|e| crate::error::PrismError::Internal(e.to_string()))?;

    if let Some(index) = op_index {
        filter_transaction_by_operation(&mut tx_data, index)?;
    }

    let error_info = host_error::classify_error(&tx_data)?;

    let mut report = report::build_report(&error_info)?;

    if error_info.is_contract_error {
        if let Ok(contract_info) = contract_error::resolve(
            &error_info.contract_id.unwrap_or_default(),
            error_info.error_code,
            network,
        )
        .await
        {
            report.contract_error = Some(contract_info);
        }
    }

    diagnostic::enrich_report(&mut report, &tx_data)?;

    context::enrich_report(&mut report, &tx_data)?;

    Ok(report)
}
